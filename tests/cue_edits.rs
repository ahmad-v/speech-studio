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
