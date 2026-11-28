//! Offline speaker diarization backed by sherpa-onnx.
//!
//! This labels speakers within one recording. It does not identify real-world
//! people. Model assets are distributed separately in an application-relative
//! `engines/speaker-diarization-v1` directory and must pass the pinned manifest
//! hashes before inference.

use localwright_core::{read_bounded, Error, Job, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sherpa_onnx::{
    FastClusteringConfig, OfflineSpeakerDiarization, OfflineSpeakerDiarizationConfig,
    OfflineSpeakerSegmentationPyannoteModelConfig, SpeakerEmbeddingExtractorConfig,
};
use std::{
    collections::BTreeSet,
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
};

pub const PACK_ID: &str = "speaker-diarization-pyannote3-3dspeaker-v1";
const SEGMENTATION_MODEL: &str = "segmentation.int8.onnx";
const EMBEDDING_MODEL: &str = "speaker-embedding.onnx";
const MAX_AUDIO_SAMPLES: usize = 9_600_000; // 10 min at 16 kHz
const MAX_SEGMENTS: usize = 100_000;
const MAX_SPEAKERS: usize = 100;
const MAX_MANIFEST_BYTES: u64 = 32 * 1024;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PackManifest {
    pub schema_version: u32,
    pub id: String,
    pub engine: String,
    pub engine_version: String,
    pub segmentation_model: String,
    pub segmentation_license: String,
    pub segmentation_source: String,
    pub embedding_model: String,
    pub embedding_license: String,
    pub embedding_source: String,
    pub segmentation_archive_size_bytes: u64,
    pub segmentation_archive_sha256: String,
    pub embedding_source_size_bytes: u64,
    pub embedding_source_sha256: String,
    pub training_data_note: String,
    pub sample_rate_hz: u32,
    pub files: Vec<PackFile>,
    pub resource_requirements: ResourceRequirements,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceRequirements {
    pub audio: String,
    pub threads: u32,
    pub model_files_bytes: u64,
    pub peak_memory: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PackFile {
    pub name: String,
    pub size_bytes: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpeakerTurn {
    pub start_ms: u64,
    pub end_ms: u64,
    /// Zero-based cluster index from sherpa-onnx, stable only within this file.
    pub speaker_index: u32,
    /// Clustering confidence when the selected backend provides it.
    pub confidence: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpeakerCue {
    pub start_ms: u64,
    pub end_ms: u64,
    pub text: String,
    /// `None` explicitly means no diarization turn overlapped this cue.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub speaker_index: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiarizedTranscript {
    pub schema_version: u32,
    pub speaker_labels: Vec<String>,
    pub segments: Vec<SpeakerCue>,
    pub diarization_engine: String,
    pub model_pack: String,
    pub limitations: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiarizationResult {
    pub schema_version: u32,
    pub speaker_count: u32,
    pub turns: Vec<SpeakerTurn>,
    pub engine: String,
    pub model_pack: String,
    pub limitations: String,
}

/// Validate persisted diarization projects before editing or exporting them.
pub fn validate_document(document: &DiarizedTranscript) -> Result<()> {
    if document.schema_version != 1
        || document.speaker_labels.len() > 100
        || document.segments.is_empty()
        || document.segments.len() > 20_000
        || document.speaker_labels.iter().any(|label| {
            let trimmed = label.trim();
            trimmed.is_empty() || trimmed.len() > 128 || trimmed.chars().any(char::is_control)
        })
        || document.segments.iter().any(|cue| {
            cue.end_ms <= cue.start_ms
                || cue.text.trim().is_empty()
                || cue.text.len() > 64 * 1024
                || cue
                    .speaker_index
                    .is_some_and(|index| index as usize >= document.speaker_labels.len())
        })
    {
        return Err(Error::input("Invalid or unsupported diarized transcript"));
    }
    Ok(())
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct DiarizationSettings {
    #[serde(default = "default_threshold")]
    threshold: f32,
    #[serde(default)]
    speaker_count: Option<u32>,
}

fn default_threshold() -> f32 {
    0.5
}

fn dense_speaker_indexes(
    raw_ids: impl IntoIterator<Item = i32>,
) -> Result<std::collections::BTreeMap<i32, u32>> {
    let ids: BTreeSet<i32> = raw_ids.into_iter().collect();
    if ids.len() > MAX_SPEAKERS || ids.iter().any(|id| *id < 0 || *id as usize >= MAX_SPEAKERS) {
        return Err(Error::input("Diarization returned an invalid speaker ID"));
    }
    Ok(ids
        .into_iter()
        .enumerate()
        .map(|(index, raw_id)| (raw_id, index as u32))
        .collect())
}

fn valid_confidence(score: f32) -> Option<f32> {
    (score.is_finite() && (-1.0..=1.0).contains(&score)).then_some(score)
}

#[derive(Debug, Clone, PartialEq)]
pub struct PackPaths {
    pub root: PathBuf,
    pub segmentation_model: PathBuf,
    pub embedding_model: PathBuf,
}

fn manifest() -> Result<PackManifest> {
    let parsed: PackManifest =
        serde_json::from_str(include_str!("../packaging/diarization/manifest.json"))?;
    if parsed.schema_version != 1
        || parsed.id != PACK_ID
        || parsed.engine != "sherpa-onnx"
        || parsed.engine_version != "2.0.3"
        || parsed.segmentation_model != "pyannote/segmentation-3.0 int8 ONNX"
        || parsed.segmentation_license != "MIT"
        || parsed.embedding_model != "3D-Speaker ERes2Net-Base (Chinese/English speaker embedding)"
        || parsed.embedding_license != "Apache-2.0"
        || parsed.segmentation_archive_size_bytes != 6_958_444
        || parsed.segmentation_archive_sha256
            != "24615ee884c897d9d2ba09bb4d30da6bb1b15e685065962db5b02e76e4996488"
        || parsed.embedding_source_size_bytes != 39_593_762
        || parsed.embedding_source_sha256
            != "f1ee61be04000b50392a63442e5dd9feaea7e81dddd494d3647cf1c3d88a1498"
        || parsed.sample_rate_hz != 16_000
        || parsed.files.len() != 5
        || parsed.training_data_note.is_empty()
        || parsed.resource_requirements.threads != 2
        || parsed.resource_requirements.model_files_bytes != 41_134_268
        || parsed.resource_requirements.audio.is_empty()
    {
        return Err(Error::input(
            "Unsupported speaker diarization model manifest",
        ));
    }
    for file in &parsed.files {
        if file.name.contains('/')
            || file.name.contains('\\')
            || file.name.is_empty()
            || file.size_bytes == 0
            || file.sha256.len() != 64
            || !file.sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(Error::input("Invalid speaker diarization file manifest"));
        }
    }
    Ok(parsed)
}

fn pack_root(app_root: &Path) -> Result<PathBuf> {
    let engines = app_root.join("engines");
    let engines_meta = fs::symlink_metadata(&engines)
        .map_err(|_| Error::input("Offline speaker diarization pack is missing"))?;
    if engines_meta.file_type().is_symlink() || !engines_meta.is_dir() {
        return Err(Error::input(
            "Offline engines path must be a regular directory",
        ));
    }
    let engines = fs::canonicalize(engines)
        .map_err(|_| Error::input("Offline speaker diarization pack is missing"))?;
    let candidate = engines.join(PACK_ID);
    let metadata = fs::symlink_metadata(&candidate)
        .map_err(|_| Error::input("Offline speaker diarization pack is missing"))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(Error::input(
            "Speaker diarization pack must be a regular directory",
        ));
    }
    let canonical = fs::canonicalize(candidate)
        .map_err(|_| Error::input("Offline speaker diarization pack is missing"))?;
    if !canonical.starts_with(engines) || !canonical.is_dir() {
        return Err(Error::input("Speaker diarization pack path is invalid"));
    }
    Ok(canonical)
}

fn hash_file(path: &Path, expected_bytes: u64, job: &Job) -> Result<String> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() != expected_bytes
    {
        return Err(Error::input(
            "Diarization model asset type or size does not match the manifest",
        ));
    }
    let mut file = File::open(path)?;
    let mut hash = Sha256::new();
    let mut total = 0_u64;
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        job.check()?;
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        total = total
            .checked_add(count as u64)
            .ok_or_else(|| Error::input("Diarization model size overflow"))?;
        if total > expected_bytes {
            return Err(Error::input(
                "Diarization model changed during verification",
            ));
        }
        hash.update(&buffer[..count]);
    }
    if total != expected_bytes {
        return Err(Error::input(
            "Diarization model changed during verification",
        ));
    }
    Ok(format!("{:x}", hash.finalize()))
}

/// Check every model and license asset before loading native inference.
pub fn verify_pack_at(app_root: &Path, job: &Job) -> Result<PackPaths> {
    let manifest = manifest()?;
    let root = pack_root(app_root)?;
    let expected: BTreeSet<_> = manifest
        .files
        .iter()
        .map(|file| file.name.clone())
        .collect();
    let mut actual = BTreeSet::new();
    for entry in fs::read_dir(&root)? {
        let entry = entry?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| Error::input("Unexpected non-UTF-8 file in diarization pack"))?;
        if !actual.insert(name) {
            return Err(Error::input("Duplicate file in diarization model pack"));
        }
    }
    if !actual.remove("manifest.json") || actual != expected {
        return Err(Error::input(
            "Diarization pack has missing or unexpected assets",
        ));
    }
    let manifest_path = root.join("manifest.json");
    let manifest_metadata = fs::symlink_metadata(&manifest_path)?;
    if !manifest_metadata.is_file()
        || manifest_metadata.file_type().is_symlink()
        || manifest_metadata.len() > MAX_MANIFEST_BYTES
    {
        return Err(Error::input("Diarization pack manifest is invalid"));
    }
    let installed_manifest: PackManifest =
        serde_json::from_slice(&read_bounded(&manifest_path, MAX_MANIFEST_BYTES)?)?;
    if installed_manifest != manifest {
        return Err(Error::input(
            "Diarization pack manifest does not match this application",
        ));
    }
    for item in &manifest.files {
        job.check()?;
        let path = root.join(&item.name);
        if hash_file(&path, item.size_bytes, job)? != item.sha256 {
            return Err(Error::input(format!(
                "Diarization pack SHA-256 mismatch: {}",
                item.name
            )));
        }
    }
    Ok(PackPaths {
        segmentation_model: root.join(SEGMENTATION_MODEL),
        embedding_model: root.join(EMBEDDING_MODEL),
        root,
    })
}

/// Find a verified pack adjacent to the application executable.
pub fn verify_installed_pack(job: &Job) -> Result<PackPaths> {
    let executable = std::env::current_exe()?;
    let app_root = executable
        .parent()
        .ok_or_else(|| Error::input("Application executable has no parent directory"))?;
    verify_pack_at(app_root, job)
}

/// Run sherpa-onnx's segmentation, embedding and clustering pipeline.
/// The native call is not interruptible internally; cancellation is checked
/// immediately before and after inference and while validating the model pack.
pub fn diarize_samples(
    samples: &[f32],
    sample_rate_hz: u32,
    pack: &PackPaths,
    threshold: f32,
    fixed_speaker_count: Option<u32>,
    job: &Job,
) -> Result<DiarizationResult> {
    if sample_rate_hz != 16_000 {
        return Err(Error::input("Diarization requires 16 kHz mono audio"));
    }
    if samples.is_empty() || samples.len() > MAX_AUDIO_SAMPLES {
        return Err(Error::input(
            "Diarization audio must be between 1 sample and 10 minutes",
        ));
    }
    if samples.iter().any(|sample| !sample.is_finite()) {
        return Err(Error::input(
            "Diarization audio contains non-finite samples",
        ));
    }
    if !threshold.is_finite() || !(0.0..=1.0).contains(&threshold) {
        return Err(Error::input(
            "Speaker clustering threshold must be between 0 and 1",
        ));
    }
    if fixed_speaker_count.is_some_and(|count| count == 0 || count > 100) {
        return Err(Error::input(
            "Fixed speaker count must be between 1 and 100",
        ));
    }
    job.check()?;
    let config = OfflineSpeakerDiarizationConfig {
        segmentation: sherpa_onnx::OfflineSpeakerSegmentationModelConfig {
            pyannote: OfflineSpeakerSegmentationPyannoteModelConfig {
                model: Some(pack.segmentation_model.to_string_lossy().into_owned()),
                ..Default::default()
            },
            num_threads: 2,
            ..Default::default()
        },
        embedding: SpeakerEmbeddingExtractorConfig {
            model: Some(pack.embedding_model.to_string_lossy().into_owned()),
            num_threads: 2,
            ..Default::default()
        },
        clustering: FastClusteringConfig {
            num_clusters: fixed_speaker_count.map_or(-1, |count| count as i32),
            threshold,
            compute_confidence: true,
        },
        ..Default::default()
    };
    let diarizer = OfflineSpeakerDiarization::create(&config).ok_or_else(|| {
        Error::input("Could not initialize the offline speaker diarization models")
    })?;
    if diarizer.sample_rate() != sample_rate_hz as i32 {
        return Err(Error::input(
            "Diarization model expects an unsupported sample rate",
        ));
    }
    job.check()?;
    let result = diarizer
        .process(samples)
        .ok_or_else(|| Error::input("Offline speaker diarization inference failed"))?;
    job.check()?;
    let reported_speaker_count = u32::try_from(result.num_speakers())
        .map_err(|_| Error::input("Diarization returned an invalid speaker count"))?;
    if reported_speaker_count as usize > MAX_SPEAKERS {
        return Err(Error::input("Diarization returned more than 100 speakers"));
    }
    let raw = result.sort_by_start_time();
    if raw.len() > MAX_SEGMENTS {
        return Err(Error::input("Diarization returned too many speaker turns"));
    }
    // sherpa reports the number of distinct emitted speaker IDs. After its
    // minimum-duration filtering those IDs need not be dense: a recording may
    // contain speaker_1 turns but no surviving speaker_0 turns. Remap the IDs
    // found in actual turns to stable dense indexes rather than rejecting a
    // valid sparse result based on `num_speakers()`.
    let speaker_indexes = dense_speaker_indexes(raw.iter().map(|turn| turn.speaker))?;
    let speaker_count = speaker_indexes.len() as u32;
    let duration_ms = (samples.len() as u64 * 1000) / sample_rate_hz as u64;
    let mut turns = Vec::with_capacity(raw.len());
    for turn in raw {
        job.check()?;
        if !turn.start.is_finite()
            || !turn.end.is_finite()
            || turn.start < 0.0
            || turn.end <= turn.start
            || turn.speaker < 0
            || turn.speaker as usize >= MAX_SPEAKERS
        {
            return Err(Error::input(format!(
                "Diarization returned an invalid segment (start={}, end={}, speaker_id={})",
                turn.start, turn.end, turn.speaker
            )));
        }
        let raw_speaker_index = *speaker_indexes
            .get(&turn.speaker)
            .ok_or_else(|| Error::input("Diarization returned an unmapped speaker ID"))?;
        let start_ms = ((turn.start as f64 * 1000.0).round() as u64).min(duration_ms);
        let end_ms = ((turn.end as f64 * 1000.0).round() as u64).min(duration_ms);
        if end_ms > start_ms {
            turns.push(SpeakerTurn {
                start_ms,
                end_ms,
                speaker_index: raw_speaker_index,
                // sherpa uses -2 as its documented "unavailable" sentinel;
                // non-finite or out-of-range backend scores also mean unknown,
                // not that the otherwise valid speaker turn should be lost.
                confidence: valid_confidence(turn.confidence),
            });
        }
    }
    Ok(DiarizationResult {
        schema_version: 1,
        speaker_count,
        turns,
        engine: "sherpa-onnx 2.0.3".into(),
        model_pack: PACK_ID.into(),
        limitations: "Speaker indices distinguish clusters within this recording only and do not identify people. Short, overlapping, noisy or acoustically similar speech can be missed or mislabeled. Review and correct labels. Confidence values are backend scores, not calibrated probabilities.".into(),
    })
}

/// Run the verified application-relative model pack on a bounded mono PCM16
/// WAV. `settings` may be blank or JSON with `threshold` (0..=1) and an
/// optional fixed `speaker_count` (1..=100).
pub fn diarize_wav(
    input: &Path,
    app_root: &Path,
    settings: &str,
    job: &Job,
) -> Result<DiarizationResult> {
    if settings.len() > 4096 {
        return Err(Error::input("Diarization settings are limited to 4 KiB"));
    }
    let parsed = if settings.trim().is_empty() {
        DiarizationSettings {
            threshold: default_threshold(),
            speaker_count: None,
        }
    } else {
        serde_json::from_str::<DiarizationSettings>(settings).map_err(|_| {
            Error::input("Diarization settings must be JSON with threshold and speaker_count")
        })?
    };
    let samples = crate::wav(input, job)?;
    let pack = verify_pack_at(app_root, job)?;
    diarize_samples(
        &samples,
        16_000,
        &pack,
        parsed.threshold,
        parsed.speaker_count,
        job,
    )
}

/// Run the installed offline pack next to the current executable.
pub fn diarize_installed_wav(input: &Path, settings: &str, job: &Job) -> Result<DiarizationResult> {
    let executable = std::env::current_exe()
        .map_err(|_| Error::input("Could not locate the Speech Studio executable"))?;
    let app_root = executable
        .parent()
        .ok_or_else(|| Error::input("Speech Studio executable has no parent directory"))?;
    diarize_wav(input, app_root, settings, job)
}

/// Run diarization and bind the resulting recording-local labels to an SRT
/// transcript. Uncovered cues stay explicitly unlabeled.
pub fn diarize_srt(
    input: &Path,
    transcript: &Path,
    settings: &str,
    job: &Job,
) -> Result<DiarizedTranscript> {
    let result = diarize_installed_wav(input, settings, job)?;
    let source = String::from_utf8(read_bounded(transcript, 2 * 1024 * 1024)?)
        .map_err(|_| Error::input("Transcript must be UTF-8 SRT"))?;
    let cues = crate::parse(&source)?;
    let labeled = label_cues(&cues, &result.turns)?;
    transcript_document(result.speaker_count, labeled)
}

/// Run the installed pack against a loaded native-workspace transcript.
#[cfg(all(feature = "gui", feature = "audio-playback"))]
pub fn diarize_cues(
    input: &Path,
    cues: &[crate::Segment],
    job: &Job,
) -> Result<DiarizedTranscript> {
    let result = diarize_installed_wav(input, "", job)?;
    let labeled = label_cues(cues, &result.turns)?;
    transcript_document(result.speaker_count, labeled)
}

/// Assign each transcript cue to the speaker with the greatest temporal
/// overlap. Cues with no detected overlap remain explicitly unlabeled.
pub fn label_cues(cues: &[crate::Segment], turns: &[SpeakerTurn]) -> Result<Vec<SpeakerCue>> {
    if cues.is_empty() || cues.len() > 20_000 || turns.len() > MAX_SEGMENTS {
        return Err(Error::input(
            "Transcript or diarization result exceeds its limit",
        ));
    }
    let mut output = Vec::with_capacity(cues.len());
    for cue in cues {
        if cue.end <= cue.start {
            return Err(Error::input("Transcript cue has invalid timing"));
        }
        let mut duration_by_speaker = std::collections::BTreeMap::<u32, u64>::new();
        for turn in turns {
            let overlap = cue
                .end
                .min(turn.end_ms)
                .saturating_sub(cue.start.max(turn.start_ms));
            if overlap > 0 {
                *duration_by_speaker.entry(turn.speaker_index).or_default() += overlap;
            }
        }
        let speaker_index = duration_by_speaker
            .into_iter()
            .max_by(|(speaker_a, duration_a), (speaker_b, duration_b)| {
                duration_a
                    .cmp(duration_b)
                    .then_with(|| speaker_b.cmp(speaker_a))
            })
            .map(|(speaker, _)| speaker);
        output.push(SpeakerCue {
            start_ms: cue.start,
            end_ms: cue.end,
            text: cue.text.clone(),
            speaker_index,
        });
    }
    Ok(output)
}

/// Build a versioned, portable speaker-labeled transcript document.
pub fn transcript_document(
    speaker_count: u32,
    segments: Vec<SpeakerCue>,
) -> Result<DiarizedTranscript> {
    if speaker_count > 100 || segments.is_empty() || segments.len() > 20_000 {
        return Err(Error::input("Diarized transcript exceeds its limits"));
    }
    let used: BTreeSet<_> = segments
        .iter()
        .filter_map(|segment| segment.speaker_index)
        .collect();
    if used.iter().any(|speaker| *speaker >= speaker_count) {
        return Err(Error::input("Transcript references an unknown speaker"));
    }
    let speaker_labels = (0..speaker_count)
        .map(|index| format!("Speaker {}", index + 1))
        .collect();
    Ok(DiarizedTranscript {
        schema_version: 1,
        speaker_labels,
        segments,
        diarization_engine: "sherpa-onnx 2.0.3".into(),
        model_pack: PACK_ID.into(),
        limitations: "Speaker labels distinguish clusters within this recording only. They do not identify people. Short, overlapping, noisy or acoustically similar speech can be missed or mislabeled; review and correct labels before publication. Overlap is resolved by assigning each transcript cue to the speaker with greatest timestamp overlap.".into(),
    })
}

/// Change a cluster label while preserving the diarization's local speaker IDs.
pub fn rename_speaker(
    document: &mut DiarizedTranscript,
    speaker_index: u32,
    label: &str,
) -> Result<()> {
    let label = label.trim();
    if speaker_index as usize >= document.speaker_labels.len()
        || label.is_empty()
        || label.len() > 128
        || label.chars().any(char::is_control)
    {
        return Err(Error::input(
            "Speaker label must be 1–128 printable characters for a known speaker",
        ));
    }
    document.speaker_labels[speaker_index as usize] = label.to_owned();
    Ok(())
}

/// Manually correct the cluster assignment for a transcript cue. `None`
/// removes an uncertain assignment without deleting the transcript text.
pub fn reassign_cue_speaker(
    document: &mut DiarizedTranscript,
    cue_index: usize,
    speaker_index: Option<u32>,
) -> Result<()> {
    if cue_index >= document.segments.len()
        || speaker_index.is_some_and(|speaker| speaker as usize >= document.speaker_labels.len())
    {
        return Err(Error::input(
            "Cue or speaker index is outside the diarized transcript",
        ));
    }
    document.segments[cue_index].speaker_index = speaker_index;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sparse_backend_ids_are_remapped_to_dense_recording_local_ids() {
        // sherpa's `num_speakers()` counts distinct emitted IDs; it does not
        // guarantee that minimum-duration filtering leaves a dense ID range.
        let map = dense_speaker_indexes([1, 1]).unwrap();
        assert_eq!(map.len(), 1);
        assert_eq!(map.get(&1), Some(&0));

        let map = dense_speaker_indexes([4, 1, 4]).unwrap();
        assert_eq!(map.len(), 2);
        assert_eq!(map.get(&1), Some(&0));
        assert_eq!(map.get(&4), Some(&1));
        assert!(dense_speaker_indexes([-1]).is_err());
        assert!(dense_speaker_indexes([MAX_SPEAKERS as i32]).is_err());
    }

    #[test]
    fn backend_confidence_sentinel_and_nonfinite_values_become_unknown() {
        assert_eq!(valid_confidence(0.7), Some(0.7));
        assert_eq!(valid_confidence(-2.0), None);
        assert_eq!(valid_confidence(f32::NAN), None);
        assert_eq!(valid_confidence(f32::INFINITY), None);
        assert_eq!(valid_confidence(1.1), None);
    }

    #[test]
    fn manifest_records_redistributable_models_and_asset_hashes() {
        let manifest = manifest().unwrap();
        assert!(manifest.segmentation_license.contains("MIT"));
        assert!(manifest.embedding_license.contains("Apache-2.0"));
        assert!(manifest
            .files
            .iter()
            .any(|file| file.name == SEGMENTATION_MODEL));
        assert!(manifest
            .files
            .iter()
            .any(|file| file.name == EMBEDDING_MODEL));
    }

    #[test]
    fn assigns_transcript_cues_by_greatest_overlap_and_leaves_gaps_unlabeled() {
        let cues = vec![
            crate::Segment {
                start: 0,
                end: 1_000,
                text: "first".into(),
            },
            crate::Segment {
                start: 1_000,
                end: 2_000,
                text: "second".into(),
            },
            crate::Segment {
                start: 4_000,
                end: 5_000,
                text: "silence".into(),
            },
        ];
        let turns = vec![
            SpeakerTurn {
                start_ms: 0,
                end_ms: 700,
                speaker_index: 0,
                confidence: Some(0.9),
            },
            SpeakerTurn {
                start_ms: 700,
                end_ms: 1_200,
                speaker_index: 1,
                confidence: Some(0.8),
            },
            SpeakerTurn {
                start_ms: 1_200,
                end_ms: 2_000,
                speaker_index: 1,
                confidence: Some(0.9),
            },
        ];
        let labeled = label_cues(&cues, &turns).unwrap();
        assert_eq!(labeled[0].speaker_index, Some(0));
        assert_eq!(labeled[1].speaker_index, Some(1));
        assert_eq!(labeled[2].speaker_index, None);
    }

    #[test]
    fn tie_break_is_deterministic_and_transcript_is_versioned() {
        let cues = vec![crate::Segment {
            start: 0,
            end: 1_000,
            text: "line".into(),
        }];
        let turns = vec![
            SpeakerTurn {
                start_ms: 0,
                end_ms: 500,
                speaker_index: 1,
                confidence: None,
            },
            SpeakerTurn {
                start_ms: 500,
                end_ms: 1_000,
                speaker_index: 0,
                confidence: None,
            },
        ];
        let value = transcript_document(2, label_cues(&cues, &turns).unwrap()).unwrap();
        assert_eq!(value.segments[0].speaker_index, Some(0));
        let encoded = serde_json::to_vec(&value).unwrap();
        let decoded: DiarizedTranscript = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(value, decoded);
        assert!(decoded.limitations.contains("do not identify people"));
    }

    #[test]
    fn invalid_times_and_unknown_speaker_references_are_rejected() {
        let cues = vec![crate::Segment {
            start: 5,
            end: 5,
            text: "bad".into(),
        }];
        assert!(label_cues(&cues, &[]).is_err());
        let invalid = SpeakerCue {
            start_ms: 1,
            end_ms: 2,
            text: "x".into(),
            speaker_index: Some(1),
        };
        assert!(transcript_document(1, vec![invalid]).is_err());
    }

    #[test]
    fn speaker_names_and_manual_cue_corrections_round_trip_in_project_json() {
        let cues = vec![crate::Segment {
            start: 100,
            end: 900,
            text: "Good morning".into(),
        }];
        let mut document = transcript_document(2, label_cues(&cues, &[]).unwrap()).unwrap();
        rename_speaker(&mut document, 1, "Interviewer").unwrap();
        reassign_cue_speaker(&mut document, 0, Some(1)).unwrap();
        assert!(rename_speaker(&mut document, 1, "\n").is_err());
        assert!(reassign_cue_speaker(&mut document, 3, Some(0)).is_err());

        let encoded = serde_json::to_vec(&document).unwrap();
        let decoded: DiarizedTranscript = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(decoded.speaker_labels[1], "Interviewer");
        assert_eq!(decoded.segments[0].speaker_index, Some(1));
        assert_eq!(decoded.segments[0].text, "Good morning");
    }

    #[test]
    fn native_operation_renames_and_reassigns_without_overwriting() {
        use localwright_core::{Job, Request};

        let directory = tempfile::tempdir().unwrap();
        let input = directory.path().join("input.json");
        let renamed = directory.path().join("renamed.json");
        let reassigned = directory.path().join("reassigned.json");
        let source = transcript_document(
            2,
            vec![SpeakerCue {
                start_ms: 0,
                end_ms: 1_000,
                text: "Hello".into(),
                speaker_index: Some(0),
            }],
        )
        .unwrap();
        fs::write(&input, serde_json::to_vec(&source).unwrap()).unwrap();

        crate::handle(
            Request {
                operation: "rename-diarized-speaker".into(),
                input: input.clone(),
                output: renamed.clone(),
                parameter: r#"{"speaker_index":1,"label":"Interviewer"}"#.into(),
                ..Request::default()
            },
            Job::new(),
        )
        .unwrap();
        crate::handle(
            Request {
                operation: "reassign-diarized-cue".into(),
                input: renamed.clone(),
                output: reassigned.clone(),
                parameter: r#"{"cue_index":0,"speaker_index":1}"#.into(),
                ..Request::default()
            },
            Job::new(),
        )
        .unwrap();

        let result: DiarizedTranscript =
            serde_json::from_slice(&fs::read(&reassigned).unwrap()).unwrap();
        assert_eq!(result.speaker_labels[1], "Interviewer");
        assert_eq!(result.segments[0].speaker_index, Some(1));
        assert_eq!(
            serde_json::from_slice::<DiarizedTranscript>(&fs::read(&input).unwrap()).unwrap(),
            source
        );
        assert!(crate::handle(
            Request {
                operation: "reassign-diarized-cue".into(),
                input: renamed,
                output: reassigned,
                parameter: r#"{"cue_index":0,"speaker_index":1}"#.into(),
                ..Request::default()
            },
            Job::new(),
        )
        .is_err());
    }
}
