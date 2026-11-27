use localwright_core::{Error, Job, Result};
use serde::Serialize;
use std::collections::HashSet;
use unicode_segmentation::UnicodeSegmentation;

const SAMPLE_RATE: usize = 16_000;
const FRAME_SAMPLES: usize = 320; // 20 ms at 16 kHz

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub(crate) struct Interval {
    pub start_ms: u64,
    pub end_ms: u64,
}

#[derive(Debug, Serialize)]
pub(crate) struct VadReport {
    schema_version: u8,
    method: &'static str,
    sample_rate_hz: u32,
    frame_ms: u8,
    pub(crate) duration_ms: u64,
    estimated_noise_floor_dbfs: f64,
    start_threshold_dbfs: f64,
    pub(crate) speech_active_ms: u64,
    pub(crate) intervals: Vec<Interval>,
    pub(crate) limitations: &'static str,
}

#[derive(Debug, Serialize)]
pub(crate) struct TranscriptReport {
    schema_version: u8,
    language: Option<LanguageEstimate>,
    pub(crate) words: usize,
    characters_excluding_whitespace: usize,
    graphemes_excluding_whitespace: usize,
    sentences: usize,
    non_empty_text_lines: usize,
    unique_word_ratio: Option<f64>,
    pub(crate) cue_count: usize,
    cue_covered_ms: u64,
    estimated_words_per_cue_minute: Option<f64>,
    asr_confidence: Option<f64>,
    pub(crate) notes: Vec<&'static str>,
}

#[derive(Debug, Serialize)]
struct LanguageEstimate {
    language: &'static str,
    iso_639_3: &'static str,
    classifier_confidence: f64,
    reliable_by_classifier: bool,
}

fn dbfs(rms: f64) -> f64 {
    if rms <= 0.0 {
        -120.0
    } else {
        (20.0 * rms.log10()).max(-120.0)
    }
}

fn frame_levels(samples: &[f32], job: &Job) -> Result<Vec<f64>> {
    let mut levels = Vec::with_capacity(samples.len().div_ceil(FRAME_SAMPLES));
    for (index, frame) in samples.chunks(FRAME_SAMPLES).enumerate() {
        if index % 256 == 0 {
            job.check()?;
        }
        if frame.is_empty() {
            continue;
        }
        let energy = frame
            .iter()
            .map(|sample| f64::from(*sample).powi(2))
            .sum::<f64>()
            / frame.len() as f64;
        levels.push(dbfs(energy.sqrt()));
    }
    Ok(levels)
}

fn activity_intervals(levels: &[f64], start_threshold: f64, job: &Job) -> Result<Vec<Interval>> {
    let release_threshold = start_threshold - 6.0;
    let mut active = Vec::with_capacity(levels.len());
    let mut in_speech = false;
    for (index, level) in levels.iter().enumerate() {
        if index % 512 == 0 {
            job.check()?;
        }
        in_speech = if in_speech {
            *level >= release_threshold
        } else {
            *level >= start_threshold
        };
        active.push(in_speech);
    }

    // Bridge short energy dips up to 120 ms, then reject clicks and other very short bursts.
    let max_gap_frames = 6;
    let mut index = 0;
    while index < active.len() {
        if active[index] {
            index += 1;
            continue;
        }
        let gap_start = index;
        while index < active.len() && !active[index] {
            index += 1;
        }
        let gap_len = index - gap_start;
        if gap_start > 0 && index < active.len() && gap_len <= max_gap_frames {
            active[gap_start..index].fill(true);
        }
    }

    let mut intervals = Vec::new();
    let mut index = 0;
    while index < active.len() {
        if !active[index] {
            index += 1;
            continue;
        }
        let start = index;
        while index < active.len() && active[index] {
            index += 1;
        }
        if index - start >= 5 {
            intervals.push(Interval {
                start_ms: (start * FRAME_SAMPLES * 1000 / SAMPLE_RATE) as u64,
                end_ms: ((index * FRAME_SAMPLES).min(levels.len() * FRAME_SAMPLES) * 1000
                    / SAMPLE_RATE) as u64,
            });
        }
    }
    Ok(intervals)
}

pub(crate) fn detect_voice_activity(samples: &[f32], job: &Job) -> Result<VadReport> {
    if samples.is_empty() {
        return Err(Error::input("Audio is empty"));
    }
    let levels = frame_levels(samples, job)?;
    let mut sorted = levels.clone();
    sorted.sort_by(f64::total_cmp);
    let noise_index = (sorted.len().saturating_sub(1) * 20) / 100;
    let noise_floor = sorted[noise_index];
    // The floor prevents digital silence from being marked active; the adaptive margin
    // suppresses steady background noise. This is an energy heuristic, not a speech model.
    let threshold = (noise_floor + 12.0).clamp(-45.0, -28.0);
    let duration_ms = samples.len() as u64 * 1000 / SAMPLE_RATE as u64;
    let mut intervals = activity_intervals(&levels, threshold, job)?;
    for interval in &mut intervals {
        interval.end_ms = interval.end_ms.min(duration_ms);
    }
    let speech_active_ms = intervals
        .iter()
        .map(|interval| interval.end_ms.saturating_sub(interval.start_ms))
        .sum();
    Ok(VadReport {
        schema_version: 1,
        method: "adaptive-rms-energy-hysteresis",
        sample_rate_hz: SAMPLE_RATE as u32,
        frame_ms: 20,
        duration_ms,
        estimated_noise_floor_dbfs: noise_floor,
        start_threshold_dbfs: threshold,
        speech_active_ms,
        intervals,
        limitations: "Energy activity only. Music, noise or tones may be marked active; quiet or unvoiced speech may be missed. This is not neural VAD or diarization.",
    })
}

fn words(text: &str) -> Vec<String> {
    UnicodeSegmentation::unicode_words(text)
        .filter(|word| word.chars().any(char::is_alphanumeric))
        .map(str::to_lowercase)
        .collect()
}

pub(crate) fn analyze_transcript(
    segments: &[crate::Segment],
    job: &Job,
) -> Result<TranscriptReport> {
    if segments.is_empty() {
        return Err(Error::input("Transcript has no cues"));
    }
    let mut text = String::new();
    for (index, segment) in segments.iter().enumerate() {
        if index % 256 == 0 {
            job.check()?;
        }
        if !text.is_empty() {
            text.push('\n');
        }
        text.push_str(&segment.text);
        if text.len() > 2 * 1024 * 1024 {
            return Err(Error::input("Transcript text exceeds 2 MiB"));
        }
    }

    let tokens = words(&text);
    let unique = tokens.iter().collect::<HashSet<_>>().len();
    let language = if text
        .chars()
        .filter(|character| character.is_alphabetic())
        .count()
        >= 24
    {
        whatlang::detect(&text).map(|info| LanguageEstimate {
            language: info.lang().eng_name(),
            iso_639_3: info.lang().code(),
            classifier_confidence: info.confidence(),
            reliable_by_classifier: info.is_reliable(),
        })
    } else {
        None
    };

    let mut ranges = segments
        .iter()
        .map(|segment| (segment.start, segment.end))
        .collect::<Vec<_>>();
    ranges.sort_unstable();
    let mut covered_ms = 0u64;
    let mut current: Option<(u64, u64)> = None;
    for (start, end) in ranges {
        current = match current {
            Some((from, to)) if start <= to => Some((from, to.max(end))),
            Some((from, to)) => {
                covered_ms = covered_ms.saturating_add(to.saturating_sub(from));
                Some((start, end))
            }
            None => Some((start, end)),
        };
    }
    if let Some((from, to)) = current {
        covered_ms = covered_ms.saturating_add(to.saturating_sub(from));
    }

    let characters = text
        .chars()
        .filter(|character| !character.is_whitespace())
        .count();
    let graphemes = text
        .graphemes(true)
        .filter(|grapheme| !grapheme.chars().all(char::is_whitespace))
        .count();
    let sentences = text
        .chars()
        .filter(|character| matches!(character, '.' | '!' | '?' | '。' | '！' | '？'))
        .count();
    let non_empty_text_lines = text
        .split(['\n', '\r'])
        .filter(|line| !line.trim().is_empty())
        .count();
    let rate = (covered_ms > 0).then(|| tokens.len() as f64 * 60_000.0 / covered_ms as f64);

    Ok(TranscriptReport {
        schema_version: 1,
        language,
        words: tokens.len(),
        characters_excluding_whitespace: characters,
        graphemes_excluding_whitespace: graphemes,
        sentences,
        non_empty_text_lines,
        unique_word_ratio: (!tokens.is_empty()).then(|| unique as f64 / tokens.len() as f64),
        cue_count: segments.len(),
        cue_covered_ms: covered_ms,
        estimated_words_per_cue_minute: rate,
        asr_confidence: None,
        notes: vec![
            "Language and confidence are estimates from the local whatlang classifier, not ASR output.",
            "No ASR confidence was supplied by the current transcription engine; no transcript confidence is inferred.",
            "Word counts use Unicode word boundaries. CJK counts and sentence punctuation may differ from linguistic tokenization.",
            "Words per cue-covered minute uses the union of subtitle cue times, not measured acoustic speech duration.",
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn synthetic_audio() -> Vec<f32> {
        let mut samples = vec![0.0; SAMPLE_RATE / 2];
        samples.extend((0..SAMPLE_RATE).map(|index| {
            let t = index as f32 / SAMPLE_RATE as f32;
            0.22 * (std::f32::consts::TAU * 180.0 * t).sin()
        }));
        samples.extend(vec![0.0; SAMPLE_RATE / 4]);
        samples.extend((0..SAMPLE_RATE / 2).map(|index| {
            let t = index as f32 / SAMPLE_RATE as f32;
            0.20 * (std::f32::consts::TAU * 220.0 * t).sin()
        }));
        samples
    }

    #[test]
    fn energy_vad_ignores_silence_and_merges_only_activity_intervals() {
        let report = detect_voice_activity(&synthetic_audio(), &Job::new()).unwrap();
        assert_eq!(report.intervals.len(), 2);
        assert!((480..=520).contains(&report.intervals[0].start_ms));
        assert!((1450..=1550).contains(&report.intervals[0].end_ms));
        assert!((1490..=1530).contains(&report.speech_active_ms));
        assert_eq!(report.method, "adaptive-rms-energy-hysteresis");
        assert!(report.limitations.contains("not neural VAD"));
    }

    #[test]
    fn silence_does_not_create_activity() {
        let report = detect_voice_activity(&vec![0.0; SAMPLE_RATE], &Job::new()).unwrap();
        assert!(report.intervals.is_empty());
        assert_eq!(report.speech_active_ms, 0);
    }

    #[test]
    fn continuous_activity_remains_detectable_without_a_silence_sample() {
        let samples = (0..SAMPLE_RATE)
            .map(|index| {
                let t = index as f32 / SAMPLE_RATE as f32;
                0.22 * (std::f32::consts::TAU * 180.0 * t).sin()
            })
            .collect::<Vec<_>>();
        let report = detect_voice_activity(&samples, &Job::new()).unwrap();
        assert_eq!(report.intervals.len(), 1);
        assert_eq!(report.intervals[0].start_ms, 0);
        assert_eq!(report.intervals[0].end_ms, 1000);
    }

    #[test]
    fn transcript_analysis_detects_language_and_never_fabricates_asr_confidence() {
        let segments = vec![
            crate::Segment {
                start: 0,
                end: 2000,
                text: "This is a small English transcript. It includes useful words!".into(),
            },
            crate::Segment {
                start: 1500,
                end: 3000,
                text: "Useful words appear again.".into(),
            },
        ];
        let report = analyze_transcript(&segments, &Job::new()).unwrap();
        let language = report.language.unwrap();
        assert_eq!(language.iso_639_3, "eng");
        assert!((0.0..=1.0).contains(&language.classifier_confidence));
        assert_eq!(report.words, 14);
        assert_eq!(report.sentences, 3);
        assert_eq!(report.cue_covered_ms, 3000);
        assert_eq!(report.asr_confidence, None);
    }

    #[test]
    fn short_and_non_latin_transcripts_are_reported_without_invented_language() {
        let short = vec![crate::Segment {
            start: 0,
            end: 1000,
            text: "Hi!".into(),
        }];
        assert!(analyze_transcript(&short, &Job::new())
            .unwrap()
            .language
            .is_none());

        let cyrillic = vec![crate::Segment {
            start: 0,
            end: 1500,
            text: "Это длинный русский текст, чтобы определить язык.".into(),
        }];
        assert_eq!(
            analyze_transcript(&cyrillic, &Job::new())
                .unwrap()
                .language
                .unwrap()
                .iso_639_3,
            "rus"
        );
    }

    #[test]
    fn word_and_speech_rate_annotations_use_unicode_and_union_of_cues() {
        let segments = vec![
            crate::Segment {
                start: 0,
                end: 2000,
                text: "Café 東京!".into(),
            },
            crate::Segment {
                start: 1000,
                end: 3000,
                text: "Café again.".into(),
            },
        ];
        let report = analyze_transcript(&segments, &Job::new()).unwrap();
        assert_eq!(report.words, 5);
        assert_eq!(report.cue_covered_ms, 3000);
        assert_eq!(report.sentences, 2);
        assert_eq!(report.non_empty_text_lines, 2);
        assert!((report.estimated_words_per_cue_minute.unwrap() - 100.0).abs() < 0.001);
    }
}
