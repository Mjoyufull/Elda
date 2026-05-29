//! Cross-command live progression event protocol.
//!
//! Handlers emit [`ProgressEvent`]s through a shared [`ProgressSink`]. The
//! same events that populate the structured `progress` array on
//! [`crate::CommandReport`] are also broadcast live, so the human surface
//! can render a tree-style frame as work happens instead of dumping the
//! full report at the end.
//!
//! The contract is intentionally small. Renderers (TTY tree, plain stream,
//! JSON event stream) live in [`crate::progress_live`] and are picked at
//! runtime based on [`crate::OutputMode`] plus terminal detection.
//!
//! A few variants here (`StepProgress`, `StepSkipped`, `FrameOutcome::Cancelled`,
//! the `ProgressUnit` units) are part of the documented runtime contract but
//! have no producer wired in yet. They are intentionally retained on the
//! enum so renderers can match exhaustively as soon as the install lane
//! emits them.
#![allow(dead_code)]

use std::sync::Arc;

/// Identifier for a live tree frame.
///
/// Frames are scoped to one command invocation; ids are unique within a
/// single `AppContext::handle` call but not stable across runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FrameId(pub u64);

/// Outcome attached to a closing [`ProgressEvent::FrameEnd`] event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameOutcome {
    Ok,
    Blocked,
    Cancelled,
}

impl FrameOutcome {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Blocked => "blocked",
            Self::Cancelled => "cancelled",
        }
    }
}

/// Event broadcast by handlers and consumed by renderers.
#[derive(Debug, Clone)]
pub enum ProgressEvent {
    FrameStart {
        frame: FrameId,
        title: String,
        subject: Option<String>,
    },
    StepStarted {
        frame: FrameId,
        step: &'static str,
        label: String,
        detail: Option<String>,
        /// When `false`, the step may share stderr with a child (e.g. `cargo`); the TTY renderer
        /// must not run the cursor-rewind spinner or assume the previous line is still this step.
        live_spinner: bool,
    },
    StepProgress {
        frame: FrameId,
        step: &'static str,
        label: String,
        current: u64,
        total: Option<u64>,
        unit: ProgressUnit,
    },
    StepDone {
        frame: FrameId,
        step: &'static str,
        label: String,
        summary: Option<String>,
    },
    StepSkipped {
        frame: FrameId,
        step: &'static str,
        label: String,
        reason: String,
    },
    StepBlocked {
        frame: FrameId,
        step: &'static str,
        label: String,
        reason: String,
        action: Option<String>,
    },
    FrameEnd {
        frame: FrameId,
        outcome: FrameOutcome,
        summary: Option<String>,
    },
}

/// Unit attached to [`ProgressEvent::StepProgress`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgressUnit {
    Bytes,
    Files,
    Items,
}

impl ProgressUnit {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Bytes => "bytes",
            Self::Files => "files",
            Self::Items => "items",
        }
    }
}

/// Sink that receives [`ProgressEvent`]s.
///
/// The default implementation is [`NullSink`], which discards events. The
/// CLI binary swaps in a live renderer based on output mode and TTY state.
pub trait ProgressSink: Send + Sync {
    fn emit(&self, event: ProgressEvent);
}

/// Sink that drops all events. Used for `--json` runs without
/// `--no-stream` not yet wired, machine readers, and tests.
#[derive(Debug, Default, Clone, Copy)]
pub struct NullSink;

impl ProgressSink for NullSink {
    fn emit(&self, _event: ProgressEvent) {}
}

/// Sink that records every event it receives. Useful for tests.
#[derive(Debug, Default)]
pub struct RecordingSink {
    events: std::sync::Mutex<Vec<ProgressEvent>>,
}

impl RecordingSink {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn snapshot(&self) -> Vec<ProgressEvent> {
        self.events.lock().expect("sink mutex").clone()
    }
}

impl ProgressSink for RecordingSink {
    fn emit(&self, event: ProgressEvent) {
        self.events.lock().expect("sink mutex").push(event);
    }
}

/// Convenience type for shared sinks owned by the [`crate::AppContext`].
pub type SharedSink = Arc<dyn ProgressSink>;

/// Allocate the next [`FrameId`] from a monotonic counter.
#[must_use]
pub fn next_frame_id(counter: &std::sync::atomic::AtomicU64) -> FrameId {
    FrameId(counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed))
}

#[cfg(test)]
mod tests {
    use super::{FrameId, FrameOutcome, NullSink, ProgressEvent, ProgressSink, RecordingSink};

    #[test]
    fn null_sink_drops_events() {
        let sink = NullSink;
        sink.emit(ProgressEvent::FrameStart {
            frame: FrameId(0),
            title: "Sync".to_owned(),
            subject: None,
        });
    }

    #[test]
    fn recording_sink_captures_event_order() {
        let sink = RecordingSink::new();
        sink.emit(ProgressEvent::FrameStart {
            frame: FrameId(0),
            title: "Sync".to_owned(),
            subject: None,
        });
        sink.emit(ProgressEvent::FrameEnd {
            frame: FrameId(0),
            outcome: FrameOutcome::Ok,
            summary: Some("done".to_owned()),
        });

        let events = sink.snapshot();
        assert_eq!(events.len(), 2);
        assert!(matches!(events[0], ProgressEvent::FrameStart { .. }));
        assert!(matches!(
            events[1],
            ProgressEvent::FrameEnd {
                outcome: FrameOutcome::Ok,
                ..
            }
        ));
    }
}
