use crate::{parse, render, Segment, Transcript};
use std::path::PathBuf;

const MAX_HISTORY: usize = 20;
const MAX_CUES: usize = 20_000;
const MAX_TIMESTAMP_MS: u64 = 100 * 60 * 60 * 1000 - 1;
const MAX_EDITED_TRANSCRIPT_BYTES: usize = 2 * 1024 * 1024;
const MAX_SYNC_OFFSET_MS: i64 = MAX_TIMESTAMP_MS as i64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TransportAction {
    Play,
    Pause,
    Resume,
}

pub(crate) fn transport_action(
    loaded: bool,
    playing: bool,
    paused: bool,
) -> Option<TransportAction> {
    if !loaded {
        None
    } else if playing && !paused {
        Some(TransportAction::Pause)
    } else if playing {
        Some(TransportAction::Resume)
    } else {
        Some(TransportAction::Play)
    }
}

pub(crate) fn clamp_seek(value_ms: f32, duration_ms: u64) -> u64 {
    if !value_ms.is_finite() || value_ms <= 0.0 {
        0
    } else {
        (value_ms.round() as u64).min(duration_ms)
    }
}

pub(crate) fn waveform_seek(x: f32, width: f32, duration_ms: u64) -> Option<u64> {
    if !x.is_finite() || !width.is_finite() || width <= 0.0 || duration_ms == 0 {
        return None;
    }
    let position = x.clamp(0.0, width) / width * duration_ms as f32;
    Some(clamp_seek(position, duration_ms))
}

pub(crate) fn format_time(milliseconds: u64) -> String {
    let total_seconds = milliseconds / 1000;
    format!(
        "{:02}:{:02}.{:03}",
        total_seconds / 60,
        total_seconds % 60,
        milliseconds % 1000
    )
}

pub(crate) fn bound_waveform(peaks: &[f32], max_bins: usize) -> Vec<f32> {
    if max_bins == 0 || peaks.is_empty() {
        return Vec::new();
    }
    if peaks.len() <= max_bins {
        return peaks
            .iter()
            .map(|value| {
                if value.is_finite() {
                    value.abs().min(1.0)
                } else {
                    0.0
                }
            })
            .collect();
    }
    (0..max_bins)
        .map(|bin| {
            let start = bin * peaks.len() / max_bins;
            let end = ((bin + 1) * peaks.len() / max_bins).max(start + 1);
            peaks[start..end]
                .iter()
                .copied()
                .filter(|value| value.is_finite())
                .map(f32::abs)
                .fold(0.0f32, f32::max)
                .min(1.0)
        })
        .collect()
}

pub(crate) fn active_cue(cues: &[Segment], position_ms: u64) -> Option<usize> {
    cues.iter()
        .position(|cue| cue.start <= position_ms && position_ms < cue.end)
}

pub(crate) fn active_cue_at_audio_position(
    cues: &[Segment],
    audio_position_ms: u64,
    transcript_offset_ms: i64,
) -> Option<usize> {
    let transcript_position =
        transcript_position_for_audio(audio_position_ms, transcript_offset_ms)?;
    active_cue(cues, transcript_position)
}

/// Converts an audio playhead position to the transcript timeline. A positive
/// offset moves transcript cues later relative to the audio.
pub(crate) fn transcript_position_for_audio(
    audio_position_ms: u64,
    transcript_offset_ms: i64,
) -> Option<u64> {
    let position = i128::from(audio_position_ms) - i128::from(transcript_offset_ms);
    u64::try_from(position).ok()
}

/// Converts a cue start to the audio timeline, clamping before-zero seeks to 0.
pub(crate) fn audio_position_for_cue(cue_position_ms: u64, transcript_offset_ms: i64) -> u64 {
    let position = i128::from(cue_position_ms) + i128::from(transcript_offset_ms);
    position.clamp(0, i128::from(u64::MAX)) as u64
}

pub(crate) fn parse_transcript_offset(value: &str) -> Result<i64, String> {
    let offset = value
        .trim()
        .parse::<i64>()
        .map_err(|_| "Enter a whole-number offset in milliseconds".to_owned())?;
    if !(-MAX_SYNC_OFFSET_MS..=MAX_SYNC_OFFSET_MS).contains(&offset) {
        return Err(format!(
            "Offset must be between -{MAX_SYNC_OFFSET_MS} and {MAX_SYNC_OFFSET_MS} ms"
        ));
    }
    Ok(offset)
}

pub(crate) fn matching_cue_indices(cues: &[Segment], query: &str) -> Vec<usize> {
    cues.iter()
        .enumerate()
        .filter_map(|(index, cue)| (query.is_empty() || cue.text.contains(query)).then_some(index))
        .collect()
}

#[cfg(test)]
fn decode_project(bytes: &[u8], is_json: bool) -> Result<Vec<Segment>, String> {
    decode_workspace_project(bytes, is_json).map(|project| project.segments)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DecodedWorkspaceProject {
    pub(crate) segments: Vec<Segment>,
    pub(crate) audio_path: Option<PathBuf>,
    pub(crate) transcript_offset_ms: i64,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct WorkspaceProjectV2 {
    schema_version: u32,
    segments: Vec<Segment>,
    #[serde(default)]
    audio_path: Option<String>,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct WorkspaceProjectV3 {
    schema_version: u32,
    segments: Vec<Segment>,
    #[serde(default)]
    audio_path: Option<String>,
    #[serde(default)]
    transcript_offset_ms: i64,
}

pub(crate) fn decode_workspace_project(
    bytes: &[u8],
    is_json: bool,
) -> Result<DecodedWorkspaceProject, String> {
    let (cues, audio_path, transcript_offset_ms) = if is_json {
        let header: serde_json::Value =
            serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
        match header
            .get("schema_version")
            .and_then(serde_json::Value::as_u64)
        {
            Some(1) => {
                let project: Transcript =
                    serde_json::from_value(header).map_err(|error| error.to_string())?;
                (project.segments, None, 0)
            }
            Some(2) => {
                let project: WorkspaceProjectV2 =
                    serde_json::from_value(header).map_err(|error| error.to_string())?;
                if project.schema_version != 2 {
                    return Err("Unsupported audio workspace project version".into());
                }
                let audio_path = project
                    .audio_path
                    .map(|path| validate_relative_audio_path(&path).map(PathBuf::from))
                    .transpose()?;
                (project.segments, audio_path, 0)
            }
            Some(3) => {
                let project: WorkspaceProjectV3 =
                    serde_json::from_value(header).map_err(|error| error.to_string())?;
                if project.schema_version != 3 {
                    return Err("Unsupported audio workspace project version".into());
                }
                validate_transcript_offset(project.transcript_offset_ms)?;
                let audio_path = project
                    .audio_path
                    .map(|path| validate_relative_audio_path(&path).map(PathBuf::from))
                    .transpose()?;
                (project.segments, audio_path, project.transcript_offset_ms)
            }
            _ => return Err("Unsupported transcript project version".into()),
        }
    } else {
        let source = std::str::from_utf8(bytes).map_err(|error| error.to_string())?;
        (parse(source).map_err(|error| error.message)?, None, 0)
    };
    if cues.len() > MAX_CUES || cues.is_empty() {
        return Err("Project must contain between 1 and 20,000 cues".into());
    }
    let draft = render(&cues);
    if parse(&draft).map_err(|error| error.message)? != cues {
        return Err("Transcript project does not round trip through SRT".into());
    }
    Ok(DecodedWorkspaceProject {
        segments: cues,
        audio_path,
        transcript_offset_ms,
    })
}

pub(crate) fn encode_project(cues: &[Segment]) -> Result<Vec<u8>, String> {
    if cues.is_empty() || cues.len() > MAX_CUES {
        return Err("Project must contain between 1 and 20,000 cues".into());
    }
    let draft = render(cues);
    if parse(&draft).map_err(|error| error.message)? != cues {
        return Err("Transcript project does not round trip through SRT".into());
    }
    serde_json::to_vec_pretty(&Transcript {
        schema_version: 1,
        segments: cues.to_vec(),
    })
    .map_err(|error| error.to_string())
}

pub(crate) fn encode_workspace_project(
    cues: &[Segment],
    audio_path: Option<&str>,
    transcript_offset_ms: i64,
) -> Result<Vec<u8>, String> {
    if cues.is_empty() || cues.len() > MAX_CUES {
        return Err("Project must contain between 1 and 20,000 cues".into());
    }
    let draft = render(cues);
    if parse(&draft).map_err(|error| error.message)? != cues {
        return Err("Transcript project does not round trip through SRT".into());
    }
    validate_transcript_offset(transcript_offset_ms)?;
    if audio_path.is_some() || transcript_offset_ms != 0 {
        if let Some(path) = audio_path {
            validate_relative_audio_path(path)?;
        }
        #[derive(serde::Serialize)]
        struct WorkspaceProject<'a> {
            schema_version: u32,
            #[serde(skip_serializing_if = "Option::is_none")]
            audio_path: Option<&'a str>,
            transcript_offset_ms: i64,
            segments: &'a [Segment],
        }
        serde_json::to_vec_pretty(&WorkspaceProject {
            schema_version: 3,
            audio_path,
            transcript_offset_ms,
            segments: cues,
        })
        .map_err(|error| error.to_string())
    } else {
        encode_project(cues)
    }
}

fn validate_transcript_offset(offset_ms: i64) -> Result<(), String> {
    if !(-MAX_SYNC_OFFSET_MS..=MAX_SYNC_OFFSET_MS).contains(&offset_ms) {
        return Err(format!(
            "Transcript offset must be between -{MAX_SYNC_OFFSET_MS} and {MAX_SYNC_OFFSET_MS} ms"
        ));
    }
    Ok(())
}

fn validate_relative_audio_path(path: &str) -> Result<&str, String> {
    let path = path.trim();
    if path.is_empty()
        || path.starts_with('/')
        || path.starts_with('\\')
        || path.as_bytes().get(1) == Some(&b':')
        || path.contains('\0')
    {
        return Err("Linked audio path must be a nonempty relative path".into());
    }
    Ok(path)
}

pub(crate) struct CueHistory {
    current: Vec<Segment>,
    undo: Vec<Vec<Segment>>,
    redo: Vec<Vec<Segment>>,
}

impl CueHistory {
    pub(crate) fn new(cues: Vec<Segment>) -> Self {
        Self {
            current: cues,
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }

    pub(crate) fn cues(&self) -> &[Segment] {
        &self.current
    }

    #[cfg(all(feature = "gui", feature = "audio-playback"))]
    pub(crate) fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    #[cfg(all(feature = "gui", feature = "audio-playback"))]
    pub(crate) fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub(crate) fn set_timing(
        &mut self,
        index: usize,
        start_ms: u64,
        end_ms: u64,
    ) -> Result<(), String> {
        let mut changed = self.current.clone();
        if index >= changed.len() {
            return Err("Select a cue before editing its timing".into());
        }
        if start_ms >= end_ms || end_ms > MAX_TIMESTAMP_MS {
            return Err("Cue end must follow its start and remain below 100 hours".into());
        }
        if (index > 0 && changed[index - 1].start > start_ms)
            || (index + 1 < changed.len() && changed[index + 1].start < start_ms)
        {
            return Err("Cue start must stay in transcript order".into());
        }
        changed[index].start = start_ms;
        changed[index].end = end_ms;
        self.commit(changed, "Cue timing would make the project invalid")
    }

    pub(crate) fn set_text(&mut self, index: usize, text: &str) -> Result<(), String> {
        let mut changed = self.current.clone();
        let cue = changed
            .get_mut(index)
            .ok_or_else(|| "Select a cue before editing its text".to_owned())?;
        if text.contains('\0') || text.trim().is_empty() {
            return Err("Cue text must contain visible text and no NUL characters".into());
        }
        cue.text = text.replace("\r\n", "\n").replace('\r', "\n");
        self.commit(changed, "Cue text would make the project invalid")
    }

    /// Replace literal, case-sensitive text in one cue or throughout the transcript.
    /// The returned count is the number of replacements, not the number of changed cues.
    pub(crate) fn replace_text(
        &mut self,
        cue_index: Option<usize>,
        find: &str,
        replacement: &str,
    ) -> Result<usize, String> {
        if find.is_empty() {
            return Err("Find text cannot be empty".into());
        }
        if find.len() > 64 * 1024 || replacement.len() > 64 * 1024 {
            return Err("Find and replacement text are limited to 64 KiB each".into());
        }
        if replacement.contains('\0') {
            return Err("Replacement text cannot contain NUL characters".into());
        }
        let mut changed = self.current.clone();
        let range = match cue_index {
            Some(index) if index < changed.len() => index..index + 1,
            Some(_) => return Err("Select an existing cue before replacing its text".into()),
            None => 0..changed.len(),
        };
        let mut replacements = 0usize;
        for cue in &mut changed[range] {
            replacements = replacements
                .checked_add(cue.text.matches(find).count())
                .ok_or_else(|| "Replacement count exceeds the limit".to_owned())?;
            cue.text = cue.text.replace(find, replacement);
            if cue.text.trim().is_empty() {
                return Err("Replacement would leave a cue without visible text".into());
            }
        }
        if replacements > 0 {
            self.commit(changed, "Replacement would make the project invalid")?;
        }
        Ok(replacements)
    }

    fn commit(&mut self, changed: Vec<Segment>, invalid_message: &str) -> Result<(), String> {
        if changed.len() > MAX_CUES {
            return Err("Transcript exceeds the 20,000 cue limit".into());
        }
        if changed == self.current {
            return Ok(());
        }
        let draft = render(&changed);
        if draft.len() > MAX_EDITED_TRANSCRIPT_BYTES {
            return Err("Edited transcript exceeds the 2 MiB limit".into());
        }
        if parse(&draft).map_err(|error| error.message)? != changed {
            return Err(invalid_message.into());
        }
        self.undo
            .push(std::mem::replace(&mut self.current, changed));
        if self.undo.len() > MAX_HISTORY {
            self.undo.remove(0);
        }
        self.redo.clear();
        Ok(())
    }

    pub(crate) fn undo(&mut self) -> bool {
        if let Some(previous) = self.undo.pop() {
            self.redo
                .push(std::mem::replace(&mut self.current, previous));
            true
        } else {
            false
        }
    }

    pub(crate) fn redo(&mut self) -> bool {
        if let Some(next) = self.redo.pop() {
            self.undo.push(std::mem::replace(&mut self.current, next));
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transport_controls_reflect_loaded_playing_and_paused_states() {
        assert_eq!(transport_action(false, false, false), None);
        assert_eq!(
            transport_action(true, false, false),
            Some(TransportAction::Play)
        );
        assert_eq!(
            transport_action(true, true, false),
            Some(TransportAction::Pause)
        );
        assert_eq!(
            transport_action(true, true, true),
            Some(TransportAction::Resume)
        );
    }

    #[test]
    fn seeking_is_finite_and_clamped_to_loaded_duration() {
        assert_eq!(clamp_seek(1250.4, 5000), 1250);
        assert_eq!(clamp_seek(9000.0, 5000), 5000);
        assert_eq!(clamp_seek(f32::NAN, 5000), 0);
        assert_eq!(clamp_seek(-1.0, 5000), 0);
    }

    #[test]
    fn waveform_navigation_maps_clicks_to_bounded_audio_positions() {
        assert_eq!(waveform_seek(50.0, 100.0, 8_000), Some(4_000));
        assert_eq!(waveform_seek(-5.0, 100.0, 8_000), Some(0));
        assert_eq!(waveform_seek(120.0, 100.0, 8_000), Some(8_000));
        assert_eq!(waveform_seek(f32::NAN, 100.0, 8_000), None);
        assert_eq!(waveform_seek(10.0, 0.0, 8_000), None);
        assert_eq!(waveform_seek(10.0, 100.0, 0), None);
    }

    #[test]
    fn waveform_data_is_bounded_and_keeps_peak_information() {
        let peaks = (0..1000)
            .map(|value| if value == 501 { 1.0 } else { 0.1 })
            .collect::<Vec<_>>();
        let bounded = bound_waveform(&peaks, 240);
        assert_eq!(bounded.len(), 240);
        assert_eq!(bounded.iter().copied().fold(0.0f32, f32::max), 1.0);
        assert!(bound_waveform(&[], 12).is_empty());
        assert!(bound_waveform(&peaks, 0).is_empty());
    }

    #[test]
    fn playback_position_uses_minutes_seconds_and_milliseconds() {
        assert_eq!(format_time(3_723_456), "62:03.456");
    }

    #[test]
    fn active_cue_highlights_the_interval_containing_playback_position() {
        let cues = crate::parse(
            "1\n00:00:00,000 --> 00:00:01,000\nOne\n\n2\n00:00:01,000 --> 00:00:02,000\nTwo",
        )
        .unwrap();
        assert_eq!(active_cue(&cues, 0), Some(0));
        assert_eq!(active_cue(&cues, 1000), Some(1));
        assert_eq!(active_cue(&cues, 2000), None);
    }

    #[test]
    fn transcript_offset_moves_cue_highlighting_and_cue_seeking_consistently() {
        let cues = crate::parse("1\n00:00:01,000 --> 00:00:02,000\nWord").unwrap();
        assert_eq!(active_cue_at_audio_position(&cues, 1400, 500), None);
        assert_eq!(active_cue_at_audio_position(&cues, 1500, 500), Some(0));
        assert_eq!(audio_position_for_cue(1000, 500), 1500);

        assert_eq!(active_cue_at_audio_position(&cues, 500, -500), Some(0));
        assert_eq!(audio_position_for_cue(1000, -500), 500);
        assert_eq!(transcript_position_for_audio(499, 500), None);
        assert_eq!(audio_position_for_cue(100, -500), 0);
    }

    #[test]
    fn transcript_offset_parser_is_signed_bounded_and_requires_integer_milliseconds() {
        assert_eq!(parse_transcript_offset("+250"), Ok(250));
        assert_eq!(parse_transcript_offset(" -250 "), Ok(-250));
        assert!(parse_transcript_offset("1.5").is_err());
        assert!(parse_transcript_offset("999999999999999999999").is_err());
        assert!(parse_transcript_offset("360000001").is_err());
    }

    #[test]
    fn transcript_filter_is_literal_case_sensitive_and_keeps_original_indices() {
        let cues = crate::parse(
            "1\n00:00:00,000 --> 00:00:01,000\nA cat\n\n2\n00:00:01,000 --> 00:00:02,000\nA Cat\n\n3\n00:00:02,000 --> 00:00:03,000\nDog",
        )
        .unwrap();
        assert_eq!(matching_cue_indices(&cues, "cat"), [0]);
        assert_eq!(matching_cue_indices(&cues, "Cat"), [1]);
        assert_eq!(matching_cue_indices(&cues, ""), [0, 1, 2]);
        assert!(matching_cue_indices(&cues, "absent").is_empty());
    }

    #[test]
    fn cue_timing_edits_validate_and_support_undo_redo() {
        let cues = crate::parse(
            "1\n00:00:00,000 --> 00:00:01,000\nOne\n\n2\n00:00:01,000 --> 00:00:02,000\nTwo",
        )
        .unwrap();
        let mut history = CueHistory::new(cues);
        assert!(history.set_timing(0, 2000, 2500).is_err());
        assert_eq!(history.cues()[0].start, 0);
        history.set_timing(0, 100, 900).unwrap();
        assert_eq!(history.cues()[0].start, 100);
        assert!(history.undo());
        assert_eq!(history.cues()[0].start, 0);
        assert!(history.redo());
        assert_eq!(history.cues()[0].start, 100);
    }

    #[test]
    fn cue_text_edit_preserves_timing_and_multiline_content_through_history() {
        let cues = crate::parse(
            "1\n00:00:00,100 --> 00:00:01,000\nHello\nthere\n\n2\n00:00:01,200 --> 00:00:02,000\nWorld",
        )
        .unwrap();
        let mut history = CueHistory::new(cues);
        history.set_text(0, "Hello\r\nagain").unwrap();
        assert_eq!(history.cues()[0].text, "Hello\nagain");
        assert_eq!(history.cues()[0].start, 100);
        assert!(history.undo());
        assert_eq!(history.cues()[0].text, "Hello\nthere");
        assert!(history.redo());
        assert_eq!(history.cues()[0].text, "Hello\nagain");
        assert!(history.set_text(0, " \n ").is_err());
        assert!(history.set_text(0, "bad\0text").is_err());
        assert_eq!(history.cues()[0].text, "Hello\nagain");
    }

    #[test]
    fn literal_replace_works_per_cue_or_globally_and_is_undoable() {
        let cues = crate::parse(
            "1\n00:00:00,100 --> 00:00:01,000\nA cat and cat\n\n2\n00:00:01,200 --> 00:00:02,000\nCat",
        )
        .unwrap();
        let mut history = CueHistory::new(cues);
        assert_eq!(history.replace_text(Some(0), "cat", "dog").unwrap(), 2);
        assert_eq!(history.cues()[0].text, "A dog and dog");
        assert_eq!(history.cues()[1].text, "Cat");
        assert_eq!(history.replace_text(None, "Cat", "cat").unwrap(), 1);
        assert_eq!(history.cues()[1].text, "cat");
        assert!(history.replace_text(None, "", "x").is_err());
        assert!(history.replace_text(None, "cat", "").is_err());
        assert!(history.undo());
        assert_eq!(history.cues()[1].text, "Cat");
        assert!(history.undo());
        assert_eq!(history.cues()[0].text, "A cat and cat");
        assert!(history.redo());
        assert_eq!(history.cues()[0].text, "A dog and dog");
    }

    #[test]
    fn transcript_projects_roundtrip_srt_and_versioned_json() {
        let cues = crate::parse(
            "1\n00:00:00,100 --> 00:00:01,000\nHello\n\n2\n00:00:01,200 --> 00:00:02,000\nWorld",
        )
        .unwrap();
        let json = encode_project(&cues).unwrap();
        assert_eq!(decode_project(&json, true).unwrap(), cues);
        assert_eq!(
            decode_project(crate::render(&cues).as_bytes(), false).unwrap(),
            cues
        );
    }

    #[test]
    fn audio_workspace_projects_roundtrip_relative_media_and_keep_legacy_json() {
        let cues = crate::parse("1\n00:00:00,100 --> 00:00:01,000\nHello").unwrap();
        let json = encode_workspace_project(&cues, Some("../media/interview.wav"), 0).unwrap();
        let project = decode_workspace_project(&json, true).unwrap();
        assert_eq!(project.segments, cues);
        assert_eq!(
            project.audio_path,
            Some(PathBuf::from("../media/interview.wav"))
        );
        assert_eq!(project.transcript_offset_ms, 0);

        let legacy = encode_project(&cues).unwrap();
        let migrated = decode_workspace_project(&legacy, true).unwrap();
        assert_eq!(migrated.segments, cues);
        assert_eq!(migrated.audio_path, None);
        assert_eq!(migrated.transcript_offset_ms, 0);
    }

    #[test]
    fn workspace_v3_roundtrips_signed_offset_and_keeps_v1_v2_readable() {
        let cues = crate::parse("1\n00:00:00,100 --> 00:00:01,000\nHello").unwrap();
        let json = encode_workspace_project(&cues, Some("../media/audio.wav"), -125).unwrap();
        let value: serde_json::Value = serde_json::from_slice(&json).unwrap();
        assert_eq!(value["schema_version"], 3);
        assert_eq!(value["transcript_offset_ms"], -125);
        let decoded = decode_workspace_project(&json, true).unwrap();
        assert_eq!(decoded.segments, cues);
        assert_eq!(
            decoded.audio_path,
            Some(PathBuf::from("../media/audio.wav"))
        );
        assert_eq!(decoded.transcript_offset_ms, -125);

        let v2 = br#"{"schema_version":2,"audio_path":"audio.wav","segments":[{"start_ms":100,"end_ms":1000,"text":"Hello"}]}"#;
        let migrated = decode_workspace_project(v2, true).unwrap();
        assert_eq!(migrated.transcript_offset_ms, 0);
        assert_eq!(migrated.audio_path, Some(PathBuf::from("audio.wav")));
        let invalid_offset = br#"{"schema_version":3,"transcript_offset_ms":400000000,"segments":[{"start_ms":100,"end_ms":1000,"text":"Hello"}]}"#;
        assert!(decode_workspace_project(invalid_offset, true).is_err());
    }

    #[test]
    fn audio_workspace_projects_reject_absolute_or_unknown_media_fields() {
        let cues = crate::parse("1\n00:00:00,100 --> 00:00:01,000\nHello").unwrap();
        assert!(encode_workspace_project(&cues, Some("/home/user/audio.wav"), 0).is_err());
        assert!(decode_workspace_project(
            br#"{"schema_version":2,"audio_path":"C:\\Users\\user\\audio.wav","segments":[{"start_ms":100,"end_ms":1000,"text":"Hello"}]}"#,
            true,
        )
        .is_err());
        assert!(decode_workspace_project(
            br#"{"schema_version":2,"audio_path":"audio.wav","segments":[{"start_ms":100,"end_ms":1000,"text":"Hello"}],"unexpected":true}"#,
            true,
        )
        .is_err());
    }
}
