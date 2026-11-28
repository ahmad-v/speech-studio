#![forbid(unsafe_code)]

//! Offline speech synthesis using sherpa-onnx and the pinned LJSpeech voice pack.

use hound::{SampleFormat, WavSpec, WavWriter};
use localwright_core::{read_bounded, Error, Job, Result};
use piper_plus_g2p::{english::EnglishPhonemizer, Phonemizer};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sherpa_onnx::{
    GenerationConfig, OfflineTts, OfflineTtsConfig, OfflineTtsModelConfig,
    OfflineTtsVitsModelConfig,
};
use std::{
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
};

const PACK_ID: &str = "tts-en-us-ljspeech-medium-int8";
const MAX_TEXT_BYTES: usize = 8 * 1024;
const MAX_OUTPUT_SECONDS: usize = 180;
const MAX_OUTPUT_SAMPLES: usize = 48_000 * MAX_OUTPUT_SECONDS;
const MAX_MANIFEST_BYTES: u64 = 32 * 1024;

#[derive(Debug, Clone, Deserialize, Serialize)]
struct PackManifest {
    schema_version: u32,
    id: String,
    version: String,
    engine_version: String,
    source_archive_sha256: String,
    source_archive_size_bytes: u64,
    files: Vec<PackFile>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct PackFile {
    name: String,
    size_bytes: u64,
    sha256: String,
}

fn manifest() -> Result<PackManifest> {
    let parsed: PackManifest =
        serde_json::from_str(include_str!("../packaging/tts-model/manifest.json"))?;
    if parsed.schema_version != 1
        || parsed.id != PACK_ID
        || parsed.engine_version != "2.0.3"
        || parsed.files.len() != 4
        || parsed.source_archive_sha256.len() != 64
        || parsed.source_archive_size_bytes != 21_090_429
    {
        return Err(Error::input("Unsupported offline TTS model manifest"));
    }
    Ok(parsed)
}

fn executable_root() -> Result<PathBuf> {
    let executable = std::env::current_exe()?;
    executable
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| Error::input("Application executable has no parent directory"))
}

fn pack_root(app_root: &Path, id: &str) -> Result<PathBuf> {
    let engines = app_root.join("engines");
    let engines_metadata = fs::symlink_metadata(&engines)
        .map_err(|_| Error::input("Offline TTS model pack is missing"))?;
    if engines_metadata.file_type().is_symlink() || !engines_metadata.is_dir() {
        return Err(Error::input(
            "Offline TTS engines path must be a regular directory",
        ));
    }
    let canonical_engines = fs::canonicalize(&engines)
        .map_err(|_| Error::input("Offline TTS model pack is missing"))?;
    let candidate = engines.join(id);
    let metadata = fs::symlink_metadata(&candidate)
        .map_err(|_| Error::input("Offline TTS model pack is missing"))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(Error::input(
            "Offline TTS model pack must be a regular directory",
        ));
    }
    let canonical = fs::canonicalize(&candidate)
        .map_err(|_| Error::input("Offline TTS model pack is missing"))?;
    if !canonical.starts_with(&canonical_engines) || !canonical.is_dir() {
        return Err(Error::input("Offline TTS model pack path is invalid"));
    }
    Ok(canonical)
}

fn hash_file(path: &Path, expected_bytes: u64, job: &Job) -> Result<String> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(Error::input("TTS model assets must be regular files"));
    }
    if metadata.len() != expected_bytes {
        return Err(Error::input(
            "TTS model asset size does not match the manifest",
        ));
    }
    let mut file = File::open(path)?;
    let mut hash = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    let mut read_bytes = 0_u64;
    loop {
        job.check()?;
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        read_bytes = read_bytes
            .checked_add(count as u64)
            .ok_or_else(|| Error::input("TTS model asset size overflow"))?;
        if read_bytes > expected_bytes {
            return Err(Error::input("TTS model asset exceeded its declared size"));
        }
        hash.update(&buffer[..count]);
    }
    if read_bytes != expected_bytes {
        return Err(Error::input("TTS model asset size changed while verifying"));
    }
    Ok(format!("{:x}", hash.finalize()))
}

fn verify_pack_at(app_root: &Path, job: &Job) -> Result<(PathBuf, PackManifest)> {
    let pack = manifest()?;
    let root = pack_root(app_root, &pack.id)?;
    verify_pack_files_at(&root, &pack, job)?;
    Ok((root, pack))
}

fn verify_pack_files_at(root: &Path, pack: &PackManifest, job: &Job) -> Result<()> {
    for file in &pack.files {
        job.check()?;
        let path = root.join(&file.name);
        if path.parent() != Some(root)
            || file.sha256.len() != 64
            || !file.sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(Error::input(
                "TTS model manifest contains an unsafe asset path",
            ));
        }
        let digest = hash_file(&path, file.size_bytes, job)?;
        if !digest.eq_ignore_ascii_case(&file.sha256) {
            return Err(Error::input(format!(
                "TTS model asset {} failed SHA-256 verification",
                file.name
            )));
        }
    }
    Ok(())
}

/// Verify all model and voice metadata assets beside the application executable.
pub(crate) fn verify_private_tts_pack(job: &Job) -> Result<String> {
    let (root, pack) = verify_pack_at(&executable_root()?, job)?;
    let _ = read_bounded(&root.join("MODEL_CARD"), MAX_MANIFEST_BYTES)?;
    Ok(format!(
        "Verified {} English single-speaker model assets ({})",
        pack.id, pack.version
    ))
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TtsRequest {
    pub(crate) text: String,
    #[serde(default = "default_speed")]
    pub(crate) speed: f32,
}

fn default_speed() -> f32 {
    1.0
}

fn validate_request(request: &TtsRequest) -> Result<()> {
    if request.text.trim().is_empty() {
        return Err(Error::input("Enter text to synthesize"));
    }
    if request.text.len() > MAX_TEXT_BYTES {
        return Err(Error::input("TTS text exceeds the 8 KiB limit"));
    }
    if request.text.contains('\0') {
        return Err(Error::input("TTS text contains a NUL character"));
    }
    if !request.speed.is_finite() || !(0.75..=1.5).contains(&request.speed) {
        return Err(Error::input("Speech speed must be between 0.75 and 1.5"));
    }
    Ok(())
}

pub(crate) fn synthesize_file(
    parameter: &str,
    output: &Path,
    job: &Job,
    progress: Option<Arc<AtomicUsize>>,
) -> Result<usize> {
    if parameter.len() > MAX_TEXT_BYTES + 512 {
        return Err(Error::input("TTS settings exceed the 8 KiB limit"));
    }
    let request: TtsRequest = serde_json::from_str(parameter)?;
    validate_request(&request)?;
    if output.extension().and_then(|extension| extension.to_str()) != Some("wav") {
        return Err(Error::input("Speech synthesis output must use a .wav file"));
    }
    if output.exists() {
        return Err(Error::input("Output already exists; choose a new WAV path"));
    }
    job.check()?;
    let (root, _) = verify_pack_at(&executable_root()?, job)?;
    let model = root.join("en_US-ljspeech-medium.onnx");
    let tokens = root.join("tokens.txt");
    let config = OfflineTtsConfig {
        model: OfflineTtsModelConfig {
            vits: OfflineTtsVitsModelConfig {
                model: Some(model.to_string_lossy().into_owned()),
                tokens: Some(tokens.to_string_lossy().into_owned()),
                ..Default::default()
            },
            num_threads: 2,
            provider: Some("cpu".into()),
            ..Default::default()
        },
        max_num_sentences: 1,
        ..Default::default()
    };
    let engine = OfflineTts::create(&config)
        .ok_or_else(|| Error::engine("Could not initialize the private TTS engine"))?;
    job.check()?;
    let phonemizer = EnglishPhonemizer::new().map_err(|error| {
        Error::engine(format!("Could not initialize English phonemizer: {error}"))
    })?;
    let (phonemes, _) = phonemizer
        .phonemize_with_prosody(&request.text)
        .map_err(|error| Error::input(format!("Could not phonemize text: {error}")))?;
    if phonemes.is_empty() {
        return Err(Error::input("No pronounceable English text was found"));
    }
    let exceeded_output_limit = Arc::new(AtomicBool::new(false));
    let callback_limit = Arc::clone(&exceeded_output_limit);
    let callback_progress = progress.clone();
    let callback_job = job.clone();
    let generation = GenerationConfig {
        speed: request.speed,
        tokens: Some(vec![phonemes]),
        ..Default::default()
    };
    let generated = engine.generate_with_config(
        "",
        &generation,
        Some(move |samples: &[f32], percent: f32| {
            let count = samples.len();
            if count > MAX_OUTPUT_SAMPLES {
                callback_limit.store(true, Ordering::Relaxed);
            }
            if let Some(progress) = &callback_progress {
                progress.store(
                    (percent.clamp(0.0, 1.0) * 100.0) as usize,
                    Ordering::Relaxed,
                );
            }
            count <= MAX_OUTPUT_SAMPLES && callback_job.check().is_ok()
        }),
    );
    job.check()?;
    if exceeded_output_limit.load(Ordering::Relaxed) {
        return Err(Error::input("TTS audio exceeded its bounded output size"));
    }
    let generated =
        generated.ok_or_else(|| Error::engine("The offline TTS engine returned no audio"))?;
    let samples = generated.samples();
    let sample_rate = generated.sample_rate();
    if samples.is_empty()
        || !(1..=48_000).contains(&sample_rate)
        || samples.len() > MAX_OUTPUT_SAMPLES
        || samples.len() as f64 / f64::from(sample_rate) > MAX_OUTPUT_SECONDS as f64
    {
        return Err(Error::input("TTS audio exceeded its bounded output size"));
    }
    if samples.iter().any(|sample| !sample.is_finite()) {
        return Err(Error::engine("TTS generated a non-finite audio sample"));
    }
    job.check()?;
    let parent = output
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;
    let file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                Error::input("Output already exists; choose a new WAV path")
            } else {
                Error::from(error)
            }
        })?;
    let write_result = (|| -> Result<()> {
        let spec = WavSpec {
            channels: 1,
            sample_rate: sample_rate as u32,
            bits_per_sample: 16,
            sample_format: SampleFormat::Int,
        };
        let mut writer =
            WavWriter::new(file, spec).map_err(|error| Error::input(error.to_string()))?;
        for (index, sample) in samples.iter().copied().enumerate() {
            if index % 4096 == 0 {
                job.check()?;
            }
            let scaled = (sample.clamp(-1.0, 1.0) * 32767.0).round() as i16;
            writer
                .write_sample(scaled)
                .map_err(|error| Error::input(error.to_string()))?;
        }
        writer
            .finalize()
            .map_err(|error| Error::input(error.to_string()))?;
        Ok(())
    })();
    if let Err(error) = write_result {
        let _ = fs::remove_file(output);
        return Err(error);
    }
    if let Some(progress) = progress {
        progress.store(100, Ordering::Relaxed);
    }
    Ok(samples.len())
}

/// CLI entry point for the transcript workflow's offline synthesis operation.
pub(crate) fn handle_request(parameter: &str, output: &Path, job: &Job) -> Result<usize> {
    synthesize_file(parameter, output, job, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tts_request_has_bounded_and_valid_speed_and_text() {
        let valid: TtsRequest = serde_json::from_str(r#"{"text":"Hello, world!"}"#).unwrap();
        assert_eq!(valid.speed, 1.0);
        validate_request(&valid).unwrap();
        for json in [
            r#"{"text":"  "}"#,
            r#"{"text":"Hi","speed":0.5}"#,
            r#"{"text":"Hi","speed":2.0}"#,
            r#"{"text":"Hi","extra":true}"#,
        ] {
            let parsed = serde_json::from_str::<TtsRequest>(json);
            assert!(
                parsed.is_err() || parsed.is_ok_and(|request| validate_request(&request).is_err())
            );
        }
        let long = TtsRequest {
            text: "x".repeat(MAX_TEXT_BYTES + 1),
            speed: 1.0,
        };
        assert!(validate_request(&long).is_err());
    }

    #[test]
    fn manifest_pins_the_voice_and_declares_four_verified_assets() {
        let manifest = manifest().unwrap();
        assert_eq!(manifest.id, PACK_ID);
        assert_eq!(manifest.version, "sherpa-onnx-2.0.3");
        assert_eq!(manifest.files.len(), 4);
        assert!(manifest
            .files
            .iter()
            .any(|file| file.name == "en_US-ljspeech-medium.onnx"));
    }

    #[test]
    fn pack_verification_checks_asset_hashes() {
        let dir = tempfile::tempdir().unwrap();
        let app = dir.path();
        let model_dir = app.join("engines").join(PACK_ID);
        fs::create_dir_all(&model_dir).unwrap();
        let fake = b"model fixture";
        fs::write(model_dir.join("en_US-ljspeech-medium.onnx"), fake).unwrap();
        fs::write(model_dir.join("tokens.txt"), b"tokens").unwrap();
        fs::write(model_dir.join("MODEL_CARD"), b"card").unwrap();
        fs::write(model_dir.join("en_US-ljspeech-medium.onnx.json"), b"config").unwrap();
        let mut manifest = manifest().unwrap();
        for file in &mut manifest.files {
            let bytes = fs::read(model_dir.join(&file.name)).unwrap();
            file.size_bytes = bytes.len() as u64;
            file.sha256 = format!("{:x}", Sha256::digest(bytes));
        }
        fs::write(
            model_dir.join("manifest.json"),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
        assert!(verify_pack_files_at(&model_dir, &manifest, &Job::new()).is_ok());
        fs::write(model_dir.join("tokens.txt"), b"tampered").unwrap();
        assert!(verify_pack_files_at(&model_dir, &manifest, &Job::new()).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn pack_verification_rejects_symlink_assets() {
        use std::os::unix::fs::symlink;

        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("pack");
        fs::create_dir(&root).unwrap();
        let outside = dir.path().join("outside");
        fs::write(&outside, b"fixture").unwrap();
        symlink(&outside, root.join("tokens.txt")).unwrap();
        let pack = PackManifest {
            schema_version: 1,
            id: PACK_ID.into(),
            version: "fixture".into(),
            engine_version: "2.0.3".into(),
            source_archive_sha256: "0".repeat(64),
            source_archive_size_bytes: 1,
            files: vec![PackFile {
                name: "tokens.txt".into(),
                size_bytes: 7,
                sha256: format!("{:x}", Sha256::digest(b"fixture")),
            }],
        };
        assert!(verify_pack_files_at(&root, &pack, &Job::new()).is_err());
    }
}
