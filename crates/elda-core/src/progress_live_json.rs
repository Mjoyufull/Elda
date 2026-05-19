//! JSON event-stream renderer for [`crate::progress::ProgressEvent`]s.
//!
//! Each event becomes one line of newline-delimited JSON written to
//! stderr. The shape mirrors the structured `progress` array on
//! [`crate::CommandReport`] so machine consumers can reconstruct the
//! same step ledger from the live stream.

use std::io::{self, Write};

use serde_json::json;

use crate::progress::ProgressEvent;

pub(crate) fn render_json(event: &ProgressEvent) {
    let value = serialize_event(event);
    let stderr = io::stderr();
    let mut handle = stderr.lock();
    if let Ok(line) = serde_json::to_string(&value) {
        let _ = writeln!(handle, "{line}");
        let _ = handle.flush();
    }
}

fn serialize_event(event: &ProgressEvent) -> serde_json::Value {
    match event {
        ProgressEvent::FrameStart {
            frame,
            title,
            subject,
        } => json!({
            "event": "frame_start",
            "frame": frame.0,
            "title": title,
            "subject": subject,
        }),
        ProgressEvent::StepStarted {
            frame,
            step,
            label,
            detail,
            live_spinner,
        } => json!({
            "event": "step_started",
            "frame": frame.0,
            "step": step,
            "label": label,
            "detail": detail,
            "live_spinner": live_spinner,
        }),
        ProgressEvent::StepProgress {
            frame,
            step,
            label,
            current,
            total,
            unit,
        } => json!({
            "event": "step_progress",
            "frame": frame.0,
            "step": step,
            "label": label,
            "current": current,
            "total": total,
            "unit": unit.as_str(),
        }),
        ProgressEvent::StepDone {
            frame,
            step,
            label,
            summary,
        } => json!({
            "event": "step_done",
            "frame": frame.0,
            "step": step,
            "label": label,
            "summary": summary,
        }),
        ProgressEvent::StepSkipped {
            frame,
            step,
            label,
            reason,
        } => json!({
            "event": "step_skipped",
            "frame": frame.0,
            "step": step,
            "label": label,
            "reason": reason,
        }),
        ProgressEvent::StepBlocked {
            frame,
            step,
            label,
            reason,
            action,
        } => json!({
            "event": "step_blocked",
            "frame": frame.0,
            "step": step,
            "label": label,
            "reason": reason,
            "action": action,
        }),
        ProgressEvent::FrameEnd {
            frame,
            outcome,
            summary,
        } => json!({
            "event": "frame_end",
            "frame": frame.0,
            "outcome": outcome.as_str(),
            "summary": summary,
        }),
    }
}
