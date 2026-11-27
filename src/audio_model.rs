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
}
