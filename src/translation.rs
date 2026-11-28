#![forbid(unsafe_code)]

//! Offline English-to-French translation with the pinned OPUS-MT Marian model.
//!
//! Model and tokenizer assets are installed beside the executable in
//! `engines/opus-mt-en-fr-v1`. The application never downloads assets.

use candle_core::{DType, Device, IndexOp, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::marian::{Config, MTModel};
use localwright_core::{Error, Job, Result};
use sentencepiece_rs::SentencePieceProcessor;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
};

const PACK_ID: &str = "opus-mt-en-fr-v1";
const MODEL_REVISION: &str = "dd7f6540a7a48a7f4db59e5c0b9c42c8eea67f18";
const MAX_MANIFEST_BYTES: u64 = 16 * 1024;
const MAX_MODEL_BYTES: u64 = 320 * 1024 * 1024;
const MAX_VOCAB_BYTES: u64 = 2 * 1024 * 1024;
const MAX_INPUT_BYTES: usize = 8 * 1024;
const MAX_INPUT_TOKENS: usize = 384;
const MAX_OUTPUT_TOKENS: usize = 384;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Manifest {
    schema_version: u32,
    id: String,
    version: String,
    model_repository: String,
    model_revision: String,
    model_license: String,
    source_model_file: String,
    source_model_sha256: String,
    conversion_toolchain: String,
    tokenizer_repository: String,
    tokenizer_revision: String,
    language_pair: String,
    files: Vec<ManifestFile>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct ManifestFile {
    name: String,
    size_bytes: u64,
    sha256: String,
}

fn expected_manifest() -> Manifest {
    Manifest {
        schema_version: 1,
        id: PACK_ID.into(),
        version: "1.0.0".into(),
        model_repository: "Helsinki-NLP/opus-mt-en-fr".into(),
        model_revision: MODEL_REVISION.into(),
        model_license: "Apache-2.0 (model card metadata at the pinned upstream revision)".into(),
        source_model_file: "pytorch_model.bin".into(),
        source_model_sha256: "cc1de10b49342ad2f33e06bc4474ddd6eaca278474903c4a8636ce15680d64de"
            .into(),
        conversion_toolchain: "PyTorch 2.5.1+cpu; safetensors 0.6.2; packaging 25.0; NumPy 1.26.4"
            .into(),
        tokenizer_repository: "Helsinki-NLP/opus-mt-en-fr".into(),
        tokenizer_revision: MODEL_REVISION.into(),
        language_pair: "English (en) to French (fr)".into(),
        files: vec![
            ManifestFile {
                name: "model.safetensors".into(),
                size_bytes: 300_803_144,
                sha256: "174014ba6a746e5d8d1f31470692131c55786dff4daf320cfb882593d674646b".into(),
            },
            ManifestFile {
                name: "config.json".into(),
                size_bytes: 1_416,
                sha256: "df92c371911b45282caac6101fe7e7f5d5e9af7f9a99356067ffe306278ce72b".into(),
            },
            ManifestFile {
                name: "source.spm".into(),
                size_bytes: 778_395,
                sha256: "173e9f493a668fe396d599e28d414a201193094e6ffd7a4678e5aab0f6d3d838".into(),
            },
            ManifestFile {
                name: "target.spm".into(),
                size_bytes: 802_397,
                sha256: "78d0e717c77053f1c4b856d8661d9cb87c64f083a35418c087b9146300e4f585".into(),
            },
            ManifestFile {
                name: "vocab.json".into(),
                size_bytes: 1_339_166,
                sha256: "945c604346ce15ce4aff9001001e7f925e336d942c4087017f191871162cbdc4".into(),
            },
            ManifestFile {
                name: "MODEL_CARD.md".into(),
                size_bytes: 1_209,
                sha256: "695f64a07925db14e3e2f9d0f2074ef8ac86e4d43939451fae5603683dba90a6".into(),
            },
            ManifestFile {
                name: "LICENSE-APACHE-2.0".into(),
                size_bytes: 11_358,
                sha256: "cfc7749b96f63bd31c3c42b5c471bf756814053e847c10f3eb003417bc523d30".into(),
            },
            ManifestFile {
                name: "THIRD_PARTY_NOTICE.txt".into(),
                size_bytes: 589,
                sha256: "0c013d0d76a8b41e7ee2341df5ee754fa485a929cf5ec9e41ca538653d99936b".into(),
            },
        ],
    }
}

fn parse_model_vocabulary(
    bytes: &[u8],
    expected_size: usize,
) -> Result<(HashMap<String, usize>, Vec<String>)> {
    let token_to_id: HashMap<String, usize> = serde_json::from_slice(bytes)
        .map_err(|error| Error::input(format!("Could not parse Marian vocab.json: {error}")))?;
    if token_to_id.len() != expected_size {
        return Err(Error::input(
            "Pinned Marian vocabulary has an unexpected number of entries",
        ));
    }
    let mut id_to_piece = vec![String::new(); expected_size];
    let mut seen = vec![false; expected_size];
    for (piece, id) in &token_to_id {
        if *id >= expected_size || seen[*id] {
            return Err(Error::input(
                "Pinned Marian vocabulary IDs are out of range or duplicated",
            ));
        }
        seen[*id] = true;
        id_to_piece[*id] = piece.clone();
    }
    if seen.iter().any(|present| !present) {
        return Err(Error::input(
            "Pinned Marian vocabulary IDs are not contiguous",
        ));
    }
    Ok((token_to_id, id_to_piece))
}

fn vocabulary_id_for_piece(piece: &str, token_to_id: &HashMap<String, usize>) -> Result<u32> {
    let id = token_to_id
        .get(piece)
        .copied()
        .ok_or_else(|| Error::input("SentencePiece token is missing from Marian vocab.json"))?;
    u32::try_from(id).map_err(|_| Error::input("Marian token ID exceeds the supported range"))
}

fn app_root() -> Result<PathBuf> {
    std::env::current_exe()?
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| Error::input("Application executable has no parent directory"))
}

fn pack_root(app_root: &Path) -> Result<PathBuf> {
    let engines = app_root.join("engines");
    let candidate = engines.join(PACK_ID);
    let engines_meta = fs::symlink_metadata(&engines)
        .map_err(|_| Error::input("Offline translation pack is not installed"))?;
    let pack_meta = fs::symlink_metadata(&candidate)
        .map_err(|_| Error::input("Offline English-to-French model pack is not installed"))?;
    if engines_meta.file_type().is_symlink()
        || !engines_meta.is_dir()
        || pack_meta.file_type().is_symlink()
        || !pack_meta.is_dir()
    {
        return Err(Error::input("Offline translation pack path is invalid"));
    }
    let root = fs::canonicalize(&candidate)?;
    if !root.starts_with(fs::canonicalize(&engines)?) {
        return Err(Error::input(
            "Offline translation pack path escapes engines",
        ));
    }
    Ok(root)
}

fn hash_file(path: &Path, expected_bytes: u64, job: &Job) -> Result<String> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() || metadata.len() != expected_bytes
    {
        return Err(Error::input(
            "Translation asset type or size does not match its manifest",
        ));
    }
    if expected_bytes > MAX_MODEL_BYTES {
        return Err(Error::input(
            "Translation model exceeds the configured asset limit",
        ));
    }
    let mut file = File::open(path)?;
    let mut digest = Sha256::new();
    let mut total = 0_u64;
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        job.check()?;
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        total = total
            .checked_add(read as u64)
            .ok_or_else(|| Error::input("Translation asset size overflow"))?;
        if total > expected_bytes {
            return Err(Error::input("Translation asset changed while verifying"));
        }
        digest.update(&buffer[..read]);
    }
    if total != expected_bytes {
        return Err(Error::input(
            "Translation asset size changed while verifying",
        ));
    }
    Ok(format!("{:x}", digest.finalize()))
}

fn verify(root: &Path, job: &Job) -> Result<Manifest> {
    let expected = expected_manifest();
    let manifest_path = root.join("manifest.json");
    let metadata = fs::symlink_metadata(&manifest_path)?;
    if metadata.file_type().is_symlink()
        || !metadata.is_file()
        || metadata.len() > MAX_MANIFEST_BYTES
    {
        return Err(Error::input("Translation manifest is missing or invalid"));
    }
    let actual: Manifest = serde_json::from_slice(&fs::read(&manifest_path)?)?;
    if actual != expected {
        return Err(Error::input(
            "Translation manifest does not match the pinned English-to-French model",
        ));
    }
    for item in &expected.files {
        job.check()?;
        let path = root.join(&item.name);
        if path.parent() != Some(root)
            || item.sha256.len() != 64
            || !item.sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(Error::input(
                "Translation manifest contains an unsafe asset path",
            ));
        }
        let digest = hash_file(&path, item.size_bytes, job)?;
        if !digest.eq_ignore_ascii_case(&item.sha256) {
            return Err(Error::input(format!(
                "Translation asset {} failed SHA-256 verification",
                item.name
            )));
        }
    }
    let card = fs::read_to_string(root.join("MODEL_CARD.md"))?;
    if !card.contains("license: apache-2.0") || !card.contains("opus-mt-en-fr") {
        return Err(Error::input(
            "Translation model card does not confirm the pinned license and model",
        ));
    }
    Ok(actual)
}

pub(crate) fn verify_private_pack(job: &Job) -> Result<String> {
    let pack = verify(&pack_root(&app_root()?)?, job)?;
    Ok(format!(
        "Verified offline {} model pack ({})",
        pack.language_pair, pack.version
    ))
}

/// Translate one bounded English paragraph with deterministic greedy decoding.
/// The current pack is English→French only; it does not call a network service.
pub(crate) fn translate_english_to_french(text: &str, job: &Job) -> Result<String> {
    if text.trim().is_empty() {
        return Err(Error::input("Enter English text to translate"));
    }
    if text.len() > MAX_INPUT_BYTES {
        return Err(Error::input("Translation input is limited to 8 KiB"));
    }
    let root = pack_root(&app_root()?)?;
    verify(&root, job)?;
    let config: Config =
        serde_json::from_slice(&fs::read(root.join("config.json"))?).map_err(|error| {
            Error::input(format!(
                "Could not parse pinned Marian configuration: {error}"
            ))
        })?;
    if config.vocab_size != 59_514
        || config.decoder_vocab_size != Some(59_514)
        || config.d_model != 512
        || config.encoder_layers != 6
        || config.decoder_layers != 6
        || config.decoder_start_token_id != 59_513
        || config.pad_token_id != 59_513
        || config.eos_token_id != 0
    {
        return Err(Error::input(
            "Pinned Marian configuration has unsupported dimensions or token IDs",
        ));
    }
    let vocab_path = root.join("vocab.json");
    let vocab_metadata = fs::symlink_metadata(&vocab_path)?;
    if vocab_metadata.file_type().is_symlink()
        || !vocab_metadata.is_file()
        || vocab_metadata.len() > MAX_VOCAB_BYTES
    {
        return Err(Error::input("Pinned Marian vocabulary file is invalid"));
    }
    let (token_to_id, id_to_piece) =
        parse_model_vocabulary(&fs::read(&vocab_path)?, config.vocab_size)?;
    if token_to_id.get("</s>") != Some(&(config.eos_token_id as usize))
        || token_to_id.get("<pad>") != Some(&(config.pad_token_id as usize))
        || token_to_id.get("<unk>") != Some(&1)
    {
        return Err(Error::input(
            "Pinned Marian vocabulary special-token IDs do not match its configuration",
        ));
    }
    let source = SentencePieceProcessor::open(root.join("source.spm"))
        .map_err(|error| Error::input(format!("Could not load English tokenizer: {error}")))?;
    let target = SentencePieceProcessor::open(root.join("target.spm"))
        .map_err(|error| Error::input(format!("Could not load French tokenizer: {error}")))?;
    let source_piece_ids = source
        .encode_to_ids(text)
        .map_err(|error| Error::input(format!("Could not tokenize English text: {error}")))?;
    let mut input_tokens = Vec::with_capacity(source_piece_ids.len() + 1);
    for source_id in source_piece_ids {
        let piece = source
            .id_to_piece(source_id)
            .map_err(|error| Error::input(format!("Could not resolve English token: {error}")))?;
        input_tokens.push(vocabulary_id_for_piece(piece, &token_to_id)?);
    }
    if input_tokens.is_empty() || input_tokens.len() > MAX_INPUT_TOKENS {
        return Err(Error::input(
            "Translation text must tokenize to 1–384 subword tokens",
        ));
    }
    let eos_id = config.eos_token_id;
    if input_tokens.last() != Some(&eos_id) {
        // HF Marian maps SentencePiece pieces through vocab.json, then appends
        // the model vocabulary's </s> ID as the source encoder's EOS marker.
        input_tokens.push(eos_id);
    }
    job.check()?;

    let device = Device::Cpu;
    let weights = root.join("model.safetensors");
    let weight_data = fs::read(&weights)?;
    let variables = VarBuilder::from_buffered_safetensors(weight_data, DType::F32, &device)
        .map_err(|error| {
            Error::input(format!("Could not load the verified Marian model: {error}"))
        })?;
    let mut model = MTModel::new(&config, variables)
        .map_err(|error| Error::input(format!("Could not initialize Marian inference: {error}")))?;
    let input_ids = Tensor::new(input_tokens.as_slice(), &device)
        .and_then(|tensor| tensor.unsqueeze(0))
        .map_err(|error| Error::input(format!("Could not prepare translation input: {error}")))?;
    let encoded = model
        .encoder()
        .forward(&input_ids, 0)
        .map_err(|error| Error::input(format!("Could not encode translation input: {error}")))?;

    let mut generated = vec![config.decoder_start_token_id as usize];
    for step in 0..MAX_OUTPUT_TOKENS {
        job.check()?;
        let current_token = generated
            .last()
            .copied()
            .ok_or_else(|| Error::input("Translation decoder lost its start token"))?;
        let decoder_ids = [current_token as u32];
        let decoder = Tensor::new(decoder_ids.as_slice(), &device)
            .and_then(|tensor| tensor.unsqueeze(0))
            .map_err(|error| {
                Error::input(format!("Could not prepare translation decoder: {error}"))
            })?;
        // Reuse Marian's incremental self-attention KV cache. Each call feeds
        // one token, so the attention mask has no future positions to exclude.
        let logits = model.decode(&decoder, &encoded, step).map_err(|error| {
            Error::input(format!(
                "Offline translation failed at token {}: {error}",
                step + 1
            ))
        })?;
        let next_scores = logits
            .i((0, 0))
            .and_then(|scores| scores.to_vec1::<f32>())
            .map_err(|error| Error::input(format!("Could not read translation scores: {error}")))?;
        let next_id = next_scores
            .iter()
            .enumerate()
            .filter(|(id, score)| *id != config.pad_token_id as usize && score.is_finite())
            .max_by(|(_, left), (_, right)| left.total_cmp(right))
            .map(|(id, _)| id)
            .ok_or_else(|| Error::input("Translation model produced no finite token scores"))?;
        if next_id == config.eos_token_id as usize {
            break;
        }
        generated.push(next_id);
        job.progress(step as u64 + 1, MAX_OUTPUT_TOKENS as u64);
    }
    if generated.len() >= MAX_OUTPUT_TOKENS {
        return Err(Error::input(
            "Translation reached the 384-token output limit",
        ));
    }
    generated.remove(0);
    let target_token_ids = generated
        .iter()
        .map(|model_id| {
            let piece = id_to_piece.get(*model_id).ok_or_else(|| {
                Error::input("Marian decoder returned a token outside vocab.json")
            })?;
            target.try_piece_to_id(piece).ok_or_else(|| {
                Error::input(format!(
                    "Marian output token {piece:?} is absent from the target SentencePiece model"
                ))
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let translated = target
        .decode_ids(&target_token_ids)
        .map_err(|error| Error::input(format!("Could not decode French output: {error}")))?;
    if translated.trim().is_empty() || translated.len() > 16 * 1024 {
        return Err(Error::input(
            "Translation model returned empty or oversized output",
        ));
    }
    job.check()?;
    Ok(translated.trim().to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pinned_manifest_describes_single_apache_licensed_direction() {
        let manifest = expected_manifest();
        assert_eq!(manifest.language_pair, "English (en) to French (fr)");
        assert_eq!(manifest.model_revision, MODEL_REVISION);
        assert!(manifest.model_license.starts_with("Apache-2.0"));
        assert_eq!(manifest.source_model_file, "pytorch_model.bin");
        assert_eq!(
            manifest.source_model_sha256,
            "cc1de10b49342ad2f33e06bc4474ddd6eaca278474903c4a8636ce15680d64de"
        );
        assert_eq!(
            manifest.conversion_toolchain,
            "PyTorch 2.5.1+cpu; safetensors 0.6.2; packaging 25.0; NumPy 1.26.4"
        );
        assert_eq!(manifest.files.len(), 8);
        assert_eq!(manifest.files[0].name, "model.safetensors");
        assert_eq!(manifest.files[0].size_bytes, 300_803_144);
        assert_eq!(manifest.files[4].name, "vocab.json");
        assert_eq!(
            manifest.files[4].sha256,
            "945c604346ce15ce4aff9001001e7f925e336d942c4087017f191871162cbdc4"
        );
        assert!(manifest.files.iter().all(|file| file.sha256.len() == 64));
    }

    #[test]
    fn sentencepiece_piece_ids_are_remapped_to_model_vocabulary_ids() {
        // SentencePiece's local ID is deliberately unrelated to Marian's
        // vocabulary ID. Passing the former straight to the model corrupted
        // both input tokens and decoded output.
        let bytes = r#"{"</s>":0,"<unk>":1,"<pad>":3,"▁bonjour":2}"#.as_bytes();
        let (token_to_id, id_to_piece) = parse_model_vocabulary(bytes, 4).unwrap();
        assert_eq!(
            vocabulary_id_for_piece("▁bonjour", &token_to_id).unwrap(),
            2
        );
        assert_eq!(id_to_piece[2], "▁bonjour");
        assert_eq!(id_to_piece[0], "</s>");
    }

    #[test]
    fn malformed_model_vocabulary_ids_are_rejected() {
        let duplicate_ids = br#"{"a":0,"b":0}"#;
        assert!(parse_model_vocabulary(duplicate_ids, 2).is_err());
        let non_contiguous_ids = br#"{"a":0,"b":2}"#;
        assert!(parse_model_vocabulary(non_contiguous_ids, 2).is_err());
        let missing_piece = HashMap::from([(String::from("known"), 1)]);
        assert!(vocabulary_id_for_piece("unknown", &missing_piece).is_err());
    }

    #[test]
    fn embedded_manifest_matches_the_distribution_manifest() {
        let distributed: Manifest =
            serde_json::from_str(include_str!("../packaging/translation-model/manifest.json"))
                .unwrap();
        assert_eq!(expected_manifest(), distributed);
    }

    #[test]
    fn translation_input_limits_and_empty_text_are_rejected_before_model_access() {
        let job = Job::new();
        assert!(translate_english_to_french("  ", &job).is_err());
        assert!(translate_english_to_french(&"a".repeat(MAX_INPUT_BYTES + 1), &job).is_err());
    }
}
