//! End-to-end coverage of the live progression event protocol wired into
//! the install path via [`crate::progress::ProgressSink`].

use std::sync::Arc;

use crate::app::AppContext;
use crate::progress::{ProgressEvent, RecordingSink};

use super::support::*;
use super::*;

#[test]
fn install_flow_emits_frame_lifecycle_with_canonical_step_ids() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");

    let binary = create_vendor_binary(tempdir.path(), "live-progress-tool");
    write_local_binary_recipe(tempdir.path(), "live-progress-tool", &binary, &[]);

    let sink = Arc::new(RecordingSink::new());
    let context = AppContext::from_root(tempdir.path(), false)
        .expect("context")
        .with_progress_sink(sink.clone());

    let request = CommandRequest::new(
        vec!["i".to_owned()],
        vec!["live-progress-tool".to_owned()],
        OutputMode::Human,
        false,
    );
    let report = context.handle(request).expect("install should succeed");

    assert_eq!(report.area, "install");
    let events = sink.snapshot();
    assert!(
        !events.is_empty(),
        "live sink received zero events for install"
    );

    let frame_start = events
        .iter()
        .find(|event| matches!(event, ProgressEvent::FrameStart { .. }))
        .expect("install frame start event should fire");
    if let ProgressEvent::FrameStart { title, subject, .. } = frame_start {
        assert!(title.starts_with("Install"), "title: {title}");
        assert_eq!(subject.as_deref(), Some("live-progress-tool"));
    }

    let step_ids: Vec<&'static str> = events
        .iter()
        .filter_map(|event| match event {
            ProgressEvent::StepStarted { step, .. } | ProgressEvent::StepDone { step, .. } => {
                Some(*step)
            }
            _ => None,
        })
        .collect();

    for required in [
        "acquire-source",
        "fetch-binary",
        "stage-payload",
        "analyze-staged-objects",
        "activate",
        "record-installed-state",
    ] {
        assert!(
            step_ids.contains(&required),
            "missing canonical step `{required}` in event sequence: {step_ids:?}"
        );
    }

    let frame_end = events
        .iter()
        .rev()
        .find_map(|event| match event {
            ProgressEvent::FrameEnd { outcome, .. } => Some(*outcome),
            _ => None,
        })
        .expect("install frame end should fire");
    assert_eq!(frame_end.as_str(), "ok");
}

#[test]
fn live_sink_does_not_replay_progress_block_in_human_render() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");

    let binary = create_vendor_binary(tempdir.path(), "no-double-render-tool");
    write_local_binary_recipe(tempdir.path(), "no-double-render-tool", &binary, &[]);

    let report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["i".to_owned()],
            vec!["no-double-render-tool".to_owned()],
            OutputMode::Human,
            false,
        ),
    )
    .expect("install should succeed");

    let rendered = crate::render_human(&report);
    assert!(
        !rendered.contains("├─ Progress"),
        "post-action human render must not duplicate the live progression: {rendered}"
    );
    assert!(
        rendered.contains("├─ Result") && rendered.contains("│  no-double-render-tool"),
        "result block missing: {rendered}"
    );
}
