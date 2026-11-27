use crate::{parse, render, Segment, Transcript};

const MAX_HISTORY: usize = 20;
const MAX_CUES: usize = 20_000;
const MAX_TIMESTAMP_MS: u64 = 100 * 60 * 60 * 1000 - 1;

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

pub(crate) fn decode_project(bytes: &[u8], is_json: bool) -> Result<Vec<Segment>, String> {
    let cues = if is_json {
        let project: Transcript =
            serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
        if project.schema_version != 1 {
            return Err("Unsupported transcript project version".into());
        }
        project.segments
    } else {
        let source = std::str::from_utf8(bytes).map_err(|error| error.to_string())?;
        parse(source).map_err(|error| error.message)?
    };
    if cues.len() > MAX_CUES || cues.is_empty() {
        return Err("Project must contain between 1 and 20,000 cues".into());
    }
    let draft = render(&cues);
    if parse(&draft).map_err(|error| error.message)? != cues {
        return Err("Transcript project does not round trip through SRT".into());
    }
    Ok(cues)
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

    pub(crate) fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

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
        let draft = render(&changed);
        if parse(&draft).map_err(|error| error.message)? != changed {
            return Err("Cue timing would make the project invalid".into());
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
}
