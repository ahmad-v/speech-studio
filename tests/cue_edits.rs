use localwright_core::{Job, Request};
use std::fs;

#[test]
fn cue_edits_work_from_an_srt_path_and_preserve_the_source() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("input.srt");
    let original = "1\n00:00:01,000 --> 00:00:03,000\nCafé talks\n\n2\n00:00:02,000 --> 00:00:04,000\nSecond\n";
    fs::write(&input, original).unwrap();
    for (operation, parameter, expected) in [
        ("scale-timing", "150", "00:00:01,500 --> 00:00:04,500"),
        ("trim-cues", "2500,3500", "00:00:02,500 --> 00:00:03,000"),
        (
            "split-cue",
            r#"{"cue":1,"at_ms":2500,"first":"Café","second":"talks"}"#,
            "3\n00:00:02,500 --> 00:00:03,000\ntalks",
        ),
        (
            "merge-cues",
            "1,2",
            "00:00:01,000 --> 00:00:04,000\nCafé talks\nSecond",
        ),
        (
            "delete-cue",
            "1",
            "1\n00:00:02,000 --> 00:00:04,000\nSecond",
        ),
    ] {
        let output = dir.path().join(format!("{operation}.srt"));
        let request = Request {
            operation: operation.into(),
            input: input.clone(),
            output: output.clone(),
            parameter: parameter.into(),
            ..Request::default()
        };
        let edited = speech_studio::handle(request.clone(), Job::new()).unwrap();
        let draft = edited.draft.unwrap();
        assert!(draft.contains(expected), "{operation}: {draft}");
        assert_eq!(fs::read_to_string(output).unwrap(), draft);
        assert!(speech_studio::handle(request, Job::new()).is_err());
    }
    for (operation, parameter) in [
        ("scale-timing", "0"),
        ("trim-cues", "0,1"),
        (
            "split-cue",
            r#"{"cue":1,"at_ms":1000,"first":"x","second":"y"}"#,
        ),
        ("merge-cues", "2,1"),
        ("delete-cue", "3"),
    ] {
        assert!(
            speech_studio::handle(
                Request {
                    operation: operation.into(),
                    input: input.clone(),
                    parameter: parameter.into(),
                    ..Request::default()
                },
                Job::new()
            )
            .is_err(),
            "{operation}"
        );
    }
    assert_eq!(fs::read_to_string(input).unwrap(), original);
}

#[test]
fn ten_additional_transcript_edits_validate_and_preserve_timing() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("input.srt");
    let source = "1\n00:00:01,000 --> 00:00:02,000\n  Hello   world  \n Next  line \n\n2\n00:00:03,000 --> 00:00:04,000\nSecond cue\n\n3\n00:00:03,000 --> 00:00:04,000\nSecond cue\n";
    fs::write(&input, source).unwrap();
    let edit = |operation: &str, parameter: &str| {
        speech_studio::handle(
            Request {
                operation: operation.into(),
                input: input.clone(),
                parameter: parameter.into(),
                ..Request::default()
            },
            Job::new(),
        )
        .unwrap_or_else(|error| panic!("{operation}: {error}"))
        .draft
        .unwrap()
    };

    let text = edit("replace-cue-text", r#"{"cue":1,"text":"Fresh text"}"#);
    assert!(text.contains("Fresh text"));
    let inserted = edit(
        "insert-cue",
        r#"{"index":2,"start_ms":2200,"end_ms":2800,"text":"Inserted"}"#,
    );
    assert!(inserted.contains("00:00:02,200 --> 00:00:02,800\nInserted"));
    let retimed = edit(
        "set-cue-timing",
        r#"{"cue":2,"start_ms":2200,"end_ms":2800}"#,
    );
    assert!(retimed.contains("00:00:02,200 --> 00:00:02,800\nSecond cue"));
    let shifted = edit(
        "shift-cue-range",
        r#"{"first":2,"last":3,"offset_ms":-500}"#,
    );
    assert!(shifted.contains("00:00:02,500 --> 00:00:03,500"));
    let deduped = edit("remove-duplicate-cues", "");
    assert_eq!(deduped.matches("Second cue").count(), 1);
    let normalized = edit("normalize-cue-whitespace", "");
    assert!(normalized.contains("Hello world\nNext line"));
    let wrapped = edit("wrap-cue-text", r#"{"max_graphemes":10}"#);
    assert!(wrapped.contains("Hello\nworld"));
    let joined = edit("join-cue-lines", "");
    assert!(joined.contains("Hello world Next line"));
    let swapped = edit("swap-cue-text", r#"{"first":1,"second":2}"#);
    assert!(swapped.contains("00:00:01,000 --> 00:00:02,000\nSecond cue"));
    assert!(swapped.contains("00:00:03,000 --> 00:00:04,000\n  Hello"));
    let replaced = edit(
        "replace-text-range",
        r#"{"first":2,"last":3,"find":"Second","replace":"Third"}"#,
    );
    assert!(replaced.contains("Third cue"));
    assert!(!replaced.contains("Second cue"));

    for (operation, parameter) in [
        (
            "insert-cue",
            r#"{"index":9,"start_ms":2000,"end_ms":2500,"text":"x"}"#,
        ),
        (
            "set-cue-timing",
            r#"{"cue":1,"start_ms":2000,"end_ms":1000}"#,
        ),
        (
            "shift-cue-range",
            r#"{"first":1,"last":1,"offset_ms":-2000}"#,
        ),
        ("wrap-cue-text", r#"{"max_graphemes":2}"#),
        (
            "replace-text-range",
            r#"{"first":1,"last":2,"find":"","replace":"x"}"#,
        ),
    ] {
        assert!(
            speech_studio::handle(
                Request {
                    operation: operation.into(),
                    input: input.clone(),
                    parameter: parameter.into(),
                    ..Request::default()
                },
                Job::new()
            )
            .is_err(),
            "{operation}"
        );
    }
    assert_eq!(fs::read_to_string(input).unwrap(), source);
}
