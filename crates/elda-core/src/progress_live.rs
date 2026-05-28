//! Live renderers for [`crate::progress::ProgressEvent`]s.
//!
//! Three modes:
//! * [`LiveSinkMode::HumanTty`]: tree-style frames with cursor-overwrite of
//!   the running step line (Triangle spinner -> `✔`).
//! * [`LiveSinkMode::HumanPlain`]: one line per event, prefixed by the
//!   appropriate glyph. Used when stdout/stderr are not a TTY.
//! * [`LiveSinkMode::JsonStream`]: newline-delimited JSON event stream
//!   (formatter lives in [`crate::progress_live_json`]).
//!
//! All modes write to stderr so the live surface never collides with the
//! structured stdout report or the `--json` document.

use std::io::{self, IsTerminal, Write};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU64, Ordering},
};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::OutputMode;
use crate::app_render_tree::{Glyph, TreeStyle};
use crate::progress::{FrameOutcome, ProgressEvent, ProgressSink, ProgressUnit};
use crate::progress_live_json::render_json;
use crate::render_style::highlight_progress_line;

const TRIANGLE_SPINNER_FRAMES: &[&str] = &["◢", "◣", "◤", "◥"];
const SPINNER_TICK_MS: u64 = 80;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LiveSinkMode {
    HumanTty,
    HumanPlain,
    JsonStream,
}

impl LiveSinkMode {
    pub(crate) fn detect(output_mode: OutputMode, no_stream: bool) -> Option<Self> {
        if no_stream {
            return None;
        }
        match output_mode {
            OutputMode::Json => Some(Self::JsonStream),
            OutputMode::Human => {
                if io::stdout().is_terminal() && io::stderr().is_terminal() {
                    Some(Self::HumanTty)
                } else {
                    Some(Self::HumanPlain)
                }
            }
        }
    }
}

#[derive(Debug)]
pub(crate) struct LiveSink {
    mode: LiveSinkMode,
    style: TreeStyle,
    state: Arc<Mutex<RenderState>>,
    generation: Arc<AtomicU64>,
}

#[derive(Debug, Default)]
struct RenderState {
    active_step: Option<ActiveStep>,
}

#[derive(Debug, Clone)]
struct ActiveStep {
    label: String,
    detail: Option<String>,
    /// When false, child processes may write to stderr after this line (e.g. Cargo); StepDone
    /// must not use cursor-rewind to replace "the previous line".
    tty_tracked: bool,
}

impl LiveSink {
    pub(crate) fn new(mode: LiveSinkMode) -> Self {
        Self {
            mode,
            style: TreeStyle::detect(),
            state: Arc::new(Mutex::new(RenderState::default())),
            generation: Arc::new(AtomicU64::new(0)),
        }
    }
}

impl ProgressSink for LiveSink {
    fn emit(&self, event: ProgressEvent) {
        match self.mode {
            LiveSinkMode::HumanTty => render_tty(self, &event),
            LiveSinkMode::HumanPlain => render_plain(self, &event),
            LiveSinkMode::JsonStream => render_json(&event),
        }
    }
}

fn render_tty(sink: &LiveSink, event: &ProgressEvent) {
    let style = sink.style;
    let connector_top = match style {
        TreeStyle::Unicode => "┌─",
        TreeStyle::Ascii => "+-",
    };
    let connector_mid = match style {
        TreeStyle::Unicode => "│",
        TreeStyle::Ascii => "|",
    };
    let connector_bot = match style {
        TreeStyle::Unicode => "└─",
        TreeStyle::Ascii => "+-",
    };

    let stderr = io::stderr();
    let mut handle = stderr.lock();
    match event {
        ProgressEvent::FrameStart { title, subject, .. } => {
            stop_spinner(sink);
            match subject {
                Some(subject) => {
                    let _ = write_progress_line(
                        &mut handle,
                        &format!("{connector_top} {title}: {subject}"),
                    );
                }
                None => {
                    let _ = write_progress_line(&mut handle, &format!("{connector_top} {title}"));
                }
            }
        }
        ProgressEvent::StepStarted {
            step,
            label,
            detail,
            live_spinner,
            ..
        } => {
            // Long-running acquire/build steps stream child output; do not cursor-rewind those lines.
            let tty_tracked = *live_spinner && !matches!(*step, "acquire-source" | "build-inner");
            let generation = start_step(sink, label.clone(), detail.clone(), tty_tracked);
            let line = format_running_step_line(
                connector_mid,
                running_glyph(style),
                label,
                detail.as_deref(),
            );
            let _ = write_progress_line(&mut handle, &line);
            if tty_tracked {
                spawn_spinner(sink, generation, connector_mid);
            }
        }
        ProgressEvent::StepProgress {
            step,
            label,
            current,
            total,
            unit,
            ..
        } => {
            if *step == "build-inner" {
                stop_spinner(sink);
                let milestone = label.trim();
                if !milestone.is_empty() {
                    let _ =
                        write_progress_line(&mut handle, &format!("{connector_mid}  {milestone}"));
                }
            } else {
                let detail = format_progress_detail_for_step(step, *current, *total, *unit);
                update_step_detail(sink, label.clone(), detail.clone());
                let line = format_running_step_line(
                    connector_mid,
                    running_glyph(style),
                    label,
                    detail.as_deref(),
                );
                if running_step_allows_tty_rewind(sink) {
                    let _ = overwrite_last_line(&mut handle);
                }
                let _ = write_progress_line(&mut handle, &line);
            }
        }
        ProgressEvent::StepDone { label, summary, .. } => {
            let rewind = running_step_allows_tty_rewind(sink);
            stop_spinner(sink);
            if rewind {
                let _ = overwrite_last_line(&mut handle);
            }
            let glyph = Glyph::Done.render(style);
            let line = format_step_line(connector_mid, glyph, label, summary.as_deref());
            let _ = write_progress_line(&mut handle, &line);
        }
        ProgressEvent::StepSkipped { label, reason, .. } => {
            let rewind = running_step_allows_tty_rewind(sink);
            stop_spinner(sink);
            if rewind {
                let _ = overwrite_last_line(&mut handle);
            }
            let glyph = Glyph::Skipped.render(style);
            let _ = write_progress_line(
                &mut handle,
                &format!("{connector_mid}  {glyph} {label}: {reason}"),
            );
        }
        ProgressEvent::StepBlocked { label, reason, .. } => {
            let rewind = running_step_allows_tty_rewind(sink);
            stop_spinner(sink);
            if rewind {
                let _ = overwrite_last_line(&mut handle);
            }
            let glyph = Glyph::Blocked.render(style);
            let _ = write_progress_line(
                &mut handle,
                &format!("{connector_mid}  {glyph} {label}: {reason}"),
            );
        }
        ProgressEvent::FrameEnd {
            outcome, summary, ..
        } => {
            stop_spinner(sink);
            let glyph = match outcome {
                FrameOutcome::Ok => Glyph::Done,
                FrameOutcome::Blocked => Glyph::Blocked,
                FrameOutcome::Cancelled => Glyph::Skipped,
            };
            let suffix = summary.as_deref().unwrap_or(outcome.as_str());
            let _ = write_progress_line(
                &mut handle,
                &format!("{connector_bot} {} {suffix}", glyph.render(style)),
            );
        }
    }
    let _ = handle.flush();
}

fn write_progress_line(handle: &mut impl Write, line: &str) -> io::Result<()> {
    writeln!(handle, "{}", highlight_progress_line(line))
}

fn render_plain(sink: &LiveSink, event: &ProgressEvent) {
    let style = sink.style;
    let stderr = io::stderr();
    let mut handle = stderr.lock();
    match event {
        ProgressEvent::FrameStart {
            title,
            subject,
            frame,
        } => {
            let _ = match subject {
                Some(subject) => writeln!(handle, "[frame:{}] start: {title}: {subject}", frame.0),
                None => writeln!(handle, "[frame:{}] start: {title}", frame.0),
            };
        }
        ProgressEvent::StepStarted {
            step, label, frame, ..
        } => {
            let _ = writeln!(
                handle,
                "[frame:{}] {} {step}: {label}",
                frame.0,
                running_glyph(style)
            );
        }
        ProgressEvent::StepProgress {
            current,
            total,
            step,
            label,
            frame,
            unit,
        } => match format_progress_detail_for_step(step, *current, *total, *unit) {
            Some(detail) => {
                let _ = writeln!(handle, "[frame:{}] .. {step}: {label}: {detail}", frame.0);
            }
            None => {
                let _ = writeln!(handle, "[frame:{}] .. {step}: {label}", frame.0);
            }
        },
        ProgressEvent::StepDone {
            step,
            label,
            summary,
            frame,
        } => {
            let suffix = summary.as_deref().unwrap_or("done");
            let _ = writeln!(
                handle,
                "[frame:{}] {} {step}: {label}: {suffix}",
                frame.0,
                Glyph::Done.render(style)
            );
        }
        ProgressEvent::StepSkipped {
            step,
            label,
            reason,
            frame,
        } => {
            let _ = writeln!(
                handle,
                "[frame:{}] {} {step}: {label}: {reason}",
                frame.0,
                Glyph::Skipped.render(style)
            );
        }
        ProgressEvent::StepBlocked {
            step,
            label,
            reason,
            frame,
            ..
        } => {
            let _ = writeln!(
                handle,
                "[frame:{}] {} {step}: {label}: {reason}",
                frame.0,
                Glyph::Blocked.render(style)
            );
        }
        ProgressEvent::FrameEnd {
            outcome,
            summary,
            frame,
        } => {
            let suffix = summary.as_deref().unwrap_or(outcome.as_str());
            let _ = writeln!(handle, "[frame:{}] end: {suffix}", frame.0);
        }
    }
    let _ = handle.flush();
}

fn running_step_allows_tty_rewind(sink: &LiveSink) -> bool {
    let state = sink.state.lock().expect("sink state");
    state
        .active_step
        .as_ref()
        .is_some_and(|step| step.tty_tracked)
}

fn start_step(sink: &LiveSink, label: String, detail: Option<String>, tty_tracked: bool) -> u64 {
    {
        let mut state = sink.state.lock().expect("sink state");
        state.active_step = Some(ActiveStep {
            label,
            detail,
            tty_tracked,
        });
    }
    sink.generation.fetch_add(1, Ordering::AcqRel) + 1
}

fn update_step_detail(sink: &LiveSink, label: String, detail: Option<String>) {
    let mut state = sink.state.lock().expect("sink state");
    let tty_tracked = state
        .active_step
        .as_ref()
        .map(|step| step.tty_tracked)
        .unwrap_or(true);
    state.active_step = Some(ActiveStep {
        label,
        detail,
        tty_tracked,
    });
}

fn stop_spinner(sink: &LiveSink) {
    sink.generation.fetch_add(1, Ordering::AcqRel);
    sink.state.lock().expect("sink state").active_step = None;
}

fn spawn_spinner(sink: &LiveSink, generation: u64, connector: &'static str) {
    if sink.style == TreeStyle::Ascii {
        return;
    }

    let state = Arc::clone(&sink.state);
    let generation_state = Arc::clone(&sink.generation);
    let style = sink.style;

    thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_millis(SPINNER_TICK_MS));
            if generation_state.load(Ordering::Acquire) != generation {
                break;
            }

            let Some(line) = active_step_line(&state, connector, style) else {
                break;
            };

            let stderr = io::stderr();
            let mut handle = stderr.lock();
            let _ = overwrite_last_line(&mut handle);
            let _ = write_progress_line(&mut handle, &line);
            let _ = handle.flush();
        }
    });
}

fn active_step_line(
    state: &Arc<Mutex<RenderState>>,
    connector: &str,
    style: TreeStyle,
) -> Option<String> {
    let state = state.lock().expect("sink state");
    let active = state.active_step.as_ref()?;
    Some(format_running_step_line(
        connector,
        running_glyph(style),
        &active.label,
        active.detail.as_deref(),
    ))
}

fn format_running_step_line(
    connector: &str,
    glyph: &str,
    label: &str,
    detail: Option<&str>,
) -> String {
    match detail {
        Some(text) if !text.is_empty() => format!("{connector}  {glyph} {label}: {text}"),
        _ => format!("{connector}  {glyph} {label}"),
    }
}

fn running_glyph(style: TreeStyle) -> &'static str {
    if style == TreeStyle::Ascii {
        return "[..]";
    }
    let frames = triangle_spinner_frames();
    let frame_tick = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() / 50)
        .unwrap_or(0);
    let frame_index = (frame_tick as usize) % frames.len();
    frames[frame_index]
}

fn triangle_spinner_frames() -> &'static [&'static str] {
    debug_assert_eq!(spinners::Spinners::Triangle.to_string(), "Triangle");
    TRIANGLE_SPINNER_FRAMES
}

fn format_step_line(connector: &str, glyph: &str, label: &str, summary: Option<&str>) -> String {
    match summary {
        Some(text) if !text.is_empty() => format!("{connector}  {glyph} {label}: {text}"),
        _ => format!("{connector}  {glyph} {label}"),
    }
}

fn format_progress_detail_for_step(
    step: &str,
    current: u64,
    total: Option<u64>,
    unit: ProgressUnit,
) -> Option<String> {
    if step == "build-inner" && total.is_none() && current == 0 {
        return None;
    }
    Some(format_progress_detail(current, total, unit))
}

fn format_progress_detail(current: u64, total: Option<u64>, unit: ProgressUnit) -> String {
    match total {
        Some(total) if total > 0 => {
            format!(
                "{} / {} {} ({:.0}%)",
                current,
                total,
                unit.as_str(),
                (current as f64 / total as f64) * 100.0
            )
        }
        _ => format!("{} {}", current, unit.as_str()),
    }
}

fn overwrite_last_line<W: Write>(handle: &mut W) -> io::Result<()> {
    // Move cursor up one line, clear it, return to column 1. The previous
    // emitted line is the running step we are flipping to a final glyph.
    write!(handle, "\x1b[1A\x1b[2K\r")
}

#[cfg(test)]
mod tests {
    use super::{
        LiveSinkMode, format_progress_detail, format_progress_detail_for_step, format_step_line,
        running_glyph, triangle_spinner_frames,
    };
    use crate::OutputMode;
    use crate::app_render_tree::TreeStyle;
    use crate::progress::ProgressUnit;

    #[test]
    fn detect_returns_none_when_no_stream_set() {
        assert!(LiveSinkMode::detect(OutputMode::Json, true).is_none());
        assert!(LiveSinkMode::detect(OutputMode::Human, true).is_none());
    }

    #[test]
    fn detect_picks_json_stream_for_json_output() {
        assert_eq!(
            LiveSinkMode::detect(OutputMode::Json, false),
            Some(LiveSinkMode::JsonStream)
        );
    }

    #[test]
    fn running_glyph_uses_triangle_spinner_frame() {
        let glyph = running_glyph(TreeStyle::Unicode);

        assert!(triangle_spinner_frames().contains(&glyph));
    }

    #[test]
    fn format_progress_detail_with_total_includes_percent() {
        let rendered = format_progress_detail(512, Some(1024), ProgressUnit::Bytes);
        assert_eq!(rendered, "512 / 1024 bytes (50%)");
    }

    #[test]
    fn format_progress_detail_without_total_omits_percent() {
        let rendered = format_progress_detail(7, None, ProgressUnit::Files);
        assert_eq!(rendered, "7 files");
    }

    #[test]
    fn build_inner_progress_omits_empty_zero_detail() {
        let rendered = format_progress_detail_for_step("build-inner", 0, None, ProgressUnit::Items);
        assert_eq!(rendered, None);
    }

    #[test]
    fn format_step_line_appends_summary_when_present() {
        let line = format_step_line("│", "✔", "fetch-source", Some("12.4 MB"));
        assert_eq!(line, "│  ✔ fetch-source: 12.4 MB");
    }

    #[test]
    fn format_step_line_omits_empty_summary() {
        let line = format_step_line("│", "✔", "fetch-source", None);
        assert_eq!(line, "│  ✔ fetch-source");
    }
}
