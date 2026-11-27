#![forbid(unsafe_code)]
mod analysis;
mod cue_edit;
use localwright_core::*;
#[cfg(feature = "asr")]
use sha2::{Digest, Sha256};
use std::{io::Cursor, path::Path};
pub const OPERATIONS:&[Operation]=&[
 Operation{id:"audio-info",label:"Audio inspection",hint:"Inspect local 16 kHz mono PCM16 WAV; maximum 10 minutes."},
 Operation{id:"detect-voice-activity",label:"Detect voice activity",hint:"Estimate active intervals from mono 16 kHz PCM16 WAV energy. Results are an energy heuristic, not neural VAD or diarization. Choose a new .json output."},
 Operation{id:"analyze-transcript",label:"Analyze transcript",hint:"Estimate language and linguistic counts from an SRT draft or file. Language confidence is from a local text classifier; no ASR confidence is invented. Choose a new .json output."},
 Operation{id:"waveform-svg",label:"Export WAV waveform",hint:"Export a peak waveform from 16 kHz mono PCM16 WAV to a new SVG."},
 Operation{id:"prepare-wav",label:"Prepare WAV for speech recognition",hint:"Convert 16, 32, 44.1 or 48 kHz mono/stereo PCM16 WAV to a new 16 kHz mono WAV."},
 Operation{id:"open-vtt",label:"Open WebVTT",hint:"Import plain-text WebVTT cues into the editable SRT draft."},
 Operation{id:"open-lrc",label:"Open LRC lyrics",hint:"Import timestamped UTF-8 LRC lines into the editable draft. Cue ends are inferred from the next line; the last cue receives a 3-second duration."},
 Operation{id:"open-json",label:"Open transcript JSON",hint:"Load a version 1 transcript JSON file into the editable draft."},
 Operation{id:"open-srt",label:"Open SRT",hint:"Load and validate UTF-8 SRT into editable draft."},
 Operation{id:"export-srt",label:"Save SRT",hint:"Validate editable draft, or load SRT from input if draft empty. New .srt output."},
 Operation{id:"replace-text",label:"Find and replace transcript text",hint:"Enter JSON with find and replace. Uses the SRT file or current draft and preserves timestamps."},
 Operation{id:"scale-timing",label:"Scale cue timing",hint:"Enter a whole percentage, e.g. 105, to scale all cue times from zero."},
 Operation{id:"trim-cues",label:"Trim cues to a time range",hint:"Enter start_ms,end_ms. Clips overlapping cues, preserving the original time origin."},
 Operation{id:"split-cue",label:"Split a subtitle cue",hint:"Enter JSON with cue, at_ms, first and second. Cue numbers start at 1."},
 Operation{id:"merge-cues",label:"Merge a range of cues",hint:"Enter first,last cue numbers. Combines their text and timing span."},
 Operation{id:"delete-cue",label:"Delete a subtitle cue",hint:"Enter the 1-based cue number to remove from the draft."},
 Operation{id:"replace-cue-text",label:"Replace cue text",hint:r#"Enter JSON {"cue":1,"text":"New cue text"}. Timing is unchanged."#},
 Operation{id:"insert-cue",label:"Insert subtitle cue",hint:r#"Enter JSON with index, start_ms, end_ms and text. Index is the 1-based insertion position; cue times must retain order."#},
 Operation{id:"set-cue-timing",label:"Set cue timing",hint:r#"Enter JSON {"cue":1,"start_ms":1000,"end_ms":2500}. Times are milliseconds; end must follow start."#},
 Operation{id:"shift-cue-range",label:"Shift a cue range",hint:r#"Enter JSON {"first":2,"last":4,"offset_ms":-100}. Shifts the selected 1-based range and rejects invalid timestamps or order."#},
 Operation{id:"remove-duplicate-cues",label:"Remove duplicate cues",hint:"Removes exact duplicate records with identical start, end, and text, preserving the first occurrence."},
 Operation{id:"normalize-cue-whitespace",label:"Normalize cue whitespace",hint:"Collapses whitespace within each line and trims line edges while preserving intentional line breaks."},
 Operation{id:"wrap-cue-text",label:"Wrap cue text",hint:r#"Enter JSON {"max_graphemes":42}. Wraps at word boundaries; a single overlong word is kept intact."#},
 Operation{id:"join-cue-lines",label:"Join cue lines",hint:"Joins each cue's lines with spaces and collapses whitespace."},
 Operation{id:"swap-cue-text",label:"Swap cue text",hint:r#"Enter JSON {"first":1,"second":2}. Swaps text only; each cue keeps its timing."#},
 Operation{id:"replace-text-range",label:"Replace text in cue range",hint:r#"Enter JSON {"first":1,"last":3,"find":"old","replace":"new"}. Case-sensitive literal replacement within selected cues."#},
 Operation{id:"shift-timing",label:"Shift subtitle timing",hint:"Enter a signed offset in milliseconds. Uses the current draft or SRT input and returns a new draft."},
 Operation{id:"export-json",label:"Save transcript JSON",hint:"Export an SRT file or the edited draft with millisecond timestamps. Choose a new .json file."},
 Operation{id:"export-txt",label:"Save text",hint:"Extract text from editable SRT draft or input SRT. New .txt output."},
 Operation{id:"export-vtt",label:"Save WebVTT",hint:"Export an SRT file or edited SRT draft as plain-text WebVTT cues. Choose a new .vtt output."},
 Operation{id:"export-lrc",label:"Save LRC lyrics",hint:"Export cue start times and text to LRC. LRC has no cue end times; multiline text is joined with spaces."},
 #[cfg(feature="asr")]
 Operation{id:"transcribe",label:"Transcribe (experimental)",hint:"16 kHz mono PCM16 WAV. Leave the parameter blank for the private English pack, or supply absolute model path | SHA256. CPU inference."}];
fn err(e: impl std::fmt::Display) -> Error {
    Error::input(e.to_string())
}
fn wav(p: &Path, j: &Job) -> Result<Vec<f32>> {
    let bytes = read_bounded(p, 20 * 1024 * 1024)?;
    let mut reader = hound::WavReader::new(Cursor::new(bytes)).map_err(err)?;
    let s = reader.spec();
    if s.channels != 1
        || s.sample_rate != 16000
        || s.bits_per_sample != 16
        || s.sample_format != hound::SampleFormat::Int
    {
        return Err(Error::input("Expected mono 16000 Hz signed PCM16 WAV"));
    }
    if reader.duration() > 9_600_000 {
        return Err(Error::input("Audio duration exceeds 10 minutes"));
    }
    let mut samples = Vec::with_capacity(reader.duration() as usize);
    for (i, v) in reader.samples::<i16>().enumerate() {
        if i % 4096 == 0 {
            j.check()?;
        }
        samples.push(v.map_err(err)? as f32 / 32768.0);
    }
    if samples.is_empty() {
        return Err(Error::input("Empty audio"));
    }
    Ok(samples)
}
fn prepare_wav(input: &Path, output: &Path, job: &Job) -> Result<u32> {
    if output.extension().and_then(|ext| ext.to_str()) != Some("wav") {
        return Err(Error::input("Expected .wav output"));
    }
    let bytes = read_bounded(input, 128 * 1024 * 1024)?;
    let mut reader = hound::WavReader::new(Cursor::new(bytes)).map_err(err)?;
    let spec = reader.spec();
    if !matches!(spec.channels, 1 | 2)
        || !matches!(spec.sample_rate, 16_000 | 32_000 | 44_100 | 48_000)
        || spec.bits_per_sample != 16
        || spec.sample_format != hound::SampleFormat::Int
    {
        return Err(Error::input(
            "Expected mono or stereo PCM16 WAV at 16, 32, 44.1 or 48 kHz",
        ));
    }
    let frames = reader.duration();
    if frames == 0 || frames > 600 * spec.sample_rate {
        return Err(Error::input(
            "Audio must be nonempty and no longer than 10 minutes",
        ));
    }
    const TARGET_RATE: u32 = 16_000;
    let channel_count = i64::from(spec.channels);
    let output_frames =
        (u64::from(frames) * u64::from(TARGET_RATE)).div_ceil(u64::from(spec.sample_rate)) as u32;
    write_new(output, |file| {
        let mut writer = hound::WavWriter::new(
            file,
            hound::WavSpec {
                channels: 1,
                sample_rate: 16_000,
                bits_per_sample: 16,
                sample_format: hound::SampleFormat::Int,
            },
        )
        .map_err(err)?;
        let mut samples = reader.samples::<i16>();
        // A source frame spans TARGET_RATE ticks and an output frame spans sample_rate ticks.
        // Weight each sample by its overlap with the output frame before averaging.
        let mut filled = 0u32;
        let mut weighted_sum = 0i64;
        for frame in 0..frames {
            if frame % 1024 == 0 {
                job.check()?;
            }
            let mut channel_sum = 0i64;
            for _ in 0..spec.channels {
                channel_sum += i64::from(
                    samples
                        .next()
                        .ok_or_else(|| Error::input("Incomplete PCM frame"))?
                        .map_err(err)?,
                );
            }
            let overlap = TARGET_RATE.min(spec.sample_rate - filled);
            weighted_sum += channel_sum * i64::from(overlap);
            filled += overlap;
            if filled == spec.sample_rate {
                writer
                    .write_sample((weighted_sum / (i64::from(filled) * channel_count)) as i16)
                    .map_err(err)?;
                filled = TARGET_RATE - overlap;
                weighted_sum = channel_sum * i64::from(filled);
            }
        }
        if filled > 0 {
            writer
                .write_sample((weighted_sum / (i64::from(filled) * channel_count)) as i16)
                .map_err(err)?;
        }
        if samples.next().is_some() {
            return Err(Error::input("WAV contains an incomplete channel frame"));
        }
        job.check()?;
        writer.finalize().map_err(err)?;
        Ok(())
    })?;
    Ok(output_frames)
}
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Segment {
    #[serde(rename = "start_ms")]
    start: u64,
    #[serde(rename = "end_ms")]
    end: u64,
    text: String,
}
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Transcript {
    schema_version: u32,
    segments: Vec<Segment>,
}

fn open_transcript(path: &Path, job: &Job) -> Result<(String, usize)> {
    let bytes = read_bounded(path, 16 * 1024 * 1024)?;
    let Transcript {
        schema_version,
        segments,
    } = serde_json::from_slice(&bytes)?;
    if schema_version != 1 {
        return Err(Error::input("Unsupported transcript JSON version"));
    }
    job.check()?;
    let draft = render(&segments);
    if parse(&draft)? != segments {
        return Err(Error::input(
            "Transcript must round trip through the SRT draft without loss",
        ));
    }
    job.check()?;
    Ok((draft, segments.len()))
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Replacement {
    find: String,
    replace: String,
}

fn replace_text(mut segments: Vec<Segment>, parameter: &str, job: &Job) -> Result<(String, usize)> {
    if parameter.len() > 64 * 1024 {
        return Err(Error::input("Replacement settings exceed the limit"));
    }
    let rule: Replacement = serde_json::from_str(parameter)?;
    if rule.find.is_empty() {
        return Err(Error::input("Find text must not be empty"));
    }
    let mut count = 0usize;
    let mut remaining = 2 * 1024 * 1024usize;
    for segment in &mut segments {
        job.check()?;
        let matches = segment.text.matches(&rule.find).count();
        let bytes = (segment.text.len() - matches * rule.find.len())
            .saturating_add(matches.saturating_mul(rule.replace.len()));
        remaining = remaining
            .checked_sub(bytes)
            .ok_or_else(|| Error::input("Replacement exceeds the transcript size limit"))?;
        if matches > 0 {
            segment.text = segment.text.replace(&rule.find, &rule.replace);
            count += matches;
        }
    }
    let draft = render(&segments);
    if parse(&draft)? != segments {
        return Err(Error::input(
            "Replacement produces text outside the supported subtitle format",
        ));
    }
    job.check()?;
    Ok((draft, count))
}

fn shift_timing(mut segments: Vec<Segment>, parameter: &str, job: &Job) -> Result<String> {
    const MAX_TIMESTAMP: u64 = 100 * 60 * 60 * 1000 - 1;
    let offset = parameter
        .trim()
        .parse::<i64>()
        .map_err(|_| Error::input("Enter a signed whole number of milliseconds"))?;
    for segment in &mut segments {
        job.check()?;
        let shift = |time: u64| {
            time.checked_add_signed(offset)
                .filter(|&value| value <= MAX_TIMESTAMP)
                .ok_or_else(|| {
                    Error::input("Shift would place a cue outside the SRT timestamp range")
                })
        };
        segment.start = shift(segment.start)?;
        segment.end = shift(segment.end)?;
    }
    let draft = render(&segments);
    parse(&draft)?;
    Ok(draft)
}

fn waveform_svg(samples: &[f32], job: &Job) -> Result<String> {
    use std::fmt::Write as _;
    const WIDTH: usize = 960;
    let width = samples.len().min(WIDTH);
    let mut svg = String::from(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 960 220" role="img"><title>Audio peak waveform</title><rect width="960" height="220" fill="white"/><path d="M0 100H960" stroke="#bbb"/>"##,
    );
    for column in 0..width {
        job.check()?;
        let start = column * samples.len() / width;
        let end = (column + 1) * samples.len() / width;
        let (low, high) = samples[start..end]
            .iter()
            .fold((0.0_f32, 0.0_f32), |(low, high), &sample| {
                (low.min(sample), high.max(sample))
            });
        let x = (column as f64 + 0.5) * WIDTH as f64 / width as f64;
        let top = 100.0 - high * 90.0;
        let bottom = 100.0 - low * 90.0;
        let _ = write!(
            svg,
            r##"<path d="M{x:.3} {top:.3}V{bottom:.3}" stroke="#182850"/>"##
        );
    }
    let duration = samples.len() as f64 / 16_000.0;
    let _ = write!(
        svg,
        r#"<text x="10" y="207" font-family="sans-serif" font-size="12">0 s</text><text x="950" y="207" text-anchor="end" font-family="sans-serif" font-size="12">{duration:.3} s</text></svg>"#
    );
    Ok(svg)
}

fn stamp(ms: u64) -> String {
    format!(
        "{:02}:{:02}:{:02},{:03}",
        ms / 3600000,
        (ms / 60000) % 60,
        (ms / 1000) % 60,
        ms % 1000
    )
}
fn parse_stamp(s: &str) -> Result<u64> {
    let v = s.split([':', ',']).collect::<Vec<_>>();
    if v.len() != 4
        || v.iter()
            .zip([2, 2, 2, 3])
            .any(|(a, n)| a.len() != n || !a.bytes().all(|b| b.is_ascii_digit()))
    {
        return Err(Error::input("Invalid SRT timestamp"));
    }
    let n = v
        .iter()
        .map(|x| x.parse::<u64>().map_err(err))
        .collect::<Result<Vec<_>>>()?;
    if n[1] >= 60 || n[2] >= 60 {
        return Err(Error::input("Invalid minute or second"));
    }
    Ok(n[0] * 3600000 + n[1] * 60000 + n[2] * 1000 + n[3])
}
fn parse(s: &str) -> Result<Vec<Segment>> {
    if s.len() > 2 * 1024 * 1024 {
        return Err(Error::input("Subtitle exceeds limit"));
    }
    let s = s.trim_start_matches('\u{feff}').replace("\r\n", "\n");
    let mut segments = vec![];
    for block in s.trim().split("\n\n") {
        if block.trim().is_empty() {
            continue;
        }
        let mut lines = block.lines();
        let index = lines
            .next()
            .ok_or_else(|| Error::input("Missing index"))?
            .trim()
            .parse::<usize>()
            .map_err(err)?;
        if index != segments.len() + 1 {
            return Err(Error::input("Expected sequential SRT indices"));
        }
        let (start, end) = lines
            .next()
            .and_then(|x| x.split_once(" --> "))
            .ok_or_else(|| Error::input("Invalid SRT timing"))?;
        let (start, end) = (parse_stamp(start)?, parse_stamp(end)?);
        let text = lines.collect::<Vec<_>>().join("\n");
        if end <= start
            || text.trim().is_empty()
            || segments.last().is_some_and(|p: &Segment| p.start > start)
        {
            return Err(Error::input("Invalid segment duration, text or order"));
        }
        segments.push(Segment { start, end, text });
    }
    if segments.is_empty() {
        return Err(Error::input("No subtitle segments"));
    }
    Ok(segments)
}
fn vtt_stamp(value: &str) -> Result<u64> {
    let (clock, milliseconds) = value
        .split_once('.')
        .ok_or_else(|| Error::input("Invalid WebVTT timestamp"))?;
    let clock = match clock.matches(':').count() {
        1 => format!("00:{clock}"),
        2 => clock.to_owned(),
        _ => return Err(Error::input("Invalid WebVTT timestamp")),
    };
    parse_stamp(&format!("{clock},{milliseconds}"))
}

fn parse_vtt(source: &str, job: &Job) -> Result<Vec<Segment>> {
    if source.len() > 2 * 1024 * 1024 {
        return Err(Error::input("Subtitle exceeds limit"));
    }
    let source = source
        .trim_start_matches('\u{feff}')
        .replace("\r\n", "\n")
        .replace('\r', "\n");
    let mut blocks = source.trim_end().split("\n\n");
    let header = blocks
        .next()
        .ok_or_else(|| Error::input("Missing WebVTT header"))?;
    if header.contains('\n')
        || !header
            .strip_prefix("WEBVTT")
            .is_some_and(|suffix| suffix.is_empty() || suffix.starts_with([' ', '\t']))
    {
        return Err(Error::input("Expected a WEBVTT header"));
    }

    let mut segments = Vec::<Segment>::new();
    for block in blocks.filter(|block| !block.trim().is_empty()) {
        job.check()?;
        let mut lines = block.lines();
        let first = lines.next().expect("nonempty cue block");
        if first == "NOTE"
            || first
                .strip_prefix("NOTE")
                .is_some_and(|suffix| suffix.starts_with([' ', '\t']))
        {
            continue;
        }
        let timing = if first.contains(" --> ") {
            first
        } else {
            lines
                .next()
                .ok_or_else(|| Error::input("Missing WebVTT cue timing"))?
        };
        let (start, end) = timing
            .split_once(" --> ")
            .ok_or_else(|| Error::input("Invalid WebVTT cue timing"))?;
        let (start, end) = (vtt_stamp(start.trim())?, vtt_stamp(end.trim())?);
        let text = lines.collect::<Vec<_>>().join("\n");
        if text.contains('<') {
            return Err(Error::input(
                "WebVTT cue markup is outside the supported subset",
            ));
        }
        let text = text
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&amp;", "&");
        if end <= start
            || text.trim().is_empty()
            || segments.last().is_some_and(|p| p.start > start)
        {
            return Err(Error::input("Invalid WebVTT cue duration, text or order"));
        }
        segments.push(Segment { start, end, text });
    }
    if segments.is_empty() {
        return Err(Error::input("No WebVTT cues"));
    }
    Ok(segments)
}

fn parse_lrc(source: &str, job: &Job) -> Result<Vec<Segment>> {
    if source.len() > 2 * 1024 * 1024 {
        return Err(Error::input("Lyrics exceed limit"));
    }
    let source = source
        .trim_start_matches('\u{feff}')
        .replace("\r\n", "\n")
        .replace('\r', "\n");
    let offset = source
        .lines()
        .rev()
        .find_map(|line| {
            line.trim()
                .strip_prefix("[offset:")
                .and_then(|v| v.strip_suffix(']'))
        })
        .map(str::parse::<i64>)
        .transpose()
        .map_err(err)?
        .unwrap_or(0);
    let mut timed = Vec::<(u64, String)>::new();
    let mut draft_budget = 0usize;
    for line in source.lines() {
        job.check()?;
        if line.len() > 16 * 1024 {
            return Err(Error::input("LRC line exceeds limit"));
        }
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if line
            .strip_prefix("[offset:")
            .and_then(|v| v.strip_suffix(']'))
            .is_some()
        {
            continue;
        }
        if ["ar:", "al:", "ti:", "au:", "re:", "ve:", "by:"]
            .iter()
            .any(|tag| line.starts_with(&format!("[{tag}")) && line.ends_with(']'))
        {
            continue;
        }
        let mut rest = line;
        let mut stamps = Vec::new();
        while let Some(tagged) = rest.strip_prefix('[') {
            let Some((tag, suffix)) = tagged.split_once(']') else {
                break;
            };
            if tag.contains(':') {
                let (clock, fraction) = tag.split_once('.').unwrap_or((tag, ""));
                let (minutes, seconds) = clock
                    .split_once(':')
                    .ok_or_else(|| Error::input("Invalid LRC timestamp"))?;
                if minutes.is_empty()
                    || !minutes.bytes().all(|b| b.is_ascii_digit())
                    || seconds.len() != 2
                    || !seconds.bytes().all(|b| b.is_ascii_digit())
                {
                    return Err(Error::input("Invalid LRC timestamp"));
                }
                let minutes = minutes.parse::<u64>().map_err(err)?;
                let seconds = seconds.parse::<u64>().map_err(err)?;
                if seconds >= 60
                    || !matches!(fraction.len(), 0..=3)
                    || !fraction.bytes().all(|b| b.is_ascii_digit())
                {
                    return Err(Error::input("Invalid LRC timestamp"));
                }
                let millis = match fraction.len() {
                    0 => 0,
                    1 => fraction.parse::<u64>().map_err(err)? * 100,
                    2 => fraction.parse::<u64>().map_err(err)? * 10,
                    _ => fraction.parse::<u64>().map_err(err)?,
                };
                let base = minutes
                    .checked_mul(60_000)
                    .and_then(|v| v.checked_add(seconds * 1000 + millis))
                    .ok_or_else(|| Error::input("LRC timestamp overflow"))?;
                let adjusted = i128::from(base) + i128::from(offset);
                if !(0..=100 * 60 * 60 * 1000 - 1).contains(&adjusted) {
                    return Err(Error::input(
                        "LRC offset is outside the supported time range",
                    ));
                }
                stamps.push(adjusted as u64);
                rest = suffix;
            } else {
                break;
            }
        }
        if stamps.is_empty() {
            return Err(Error::input("Expected timestamped LRC lyric line"));
        }
        if rest.trim().is_empty() {
            return Err(Error::input("LRC lyric text is empty"));
        }
        if timed.len().saturating_add(stamps.len()) > 20_000 {
            return Err(Error::input("LRC cue count exceeds 20,000"));
        }
        draft_budget = draft_budget
            .checked_add(rest.len().saturating_mul(stamps.len()))
            .and_then(|bytes| bytes.checked_add(64 * stamps.len()))
            .filter(|&bytes| bytes <= 2 * 1024 * 1024)
            .ok_or_else(|| Error::input("Expanded LRC draft exceeds 2 MiB"))?;
        timed.extend(stamps.into_iter().map(|time| (time, rest.to_owned())));
    }
    timed.sort_by_key(|(time, _)| *time);
    if timed.is_empty() || timed.windows(2).any(|pair| pair[0].0 >= pair[1].0) {
        return Err(Error::input("LRC timestamps must be unique and increasing"));
    }
    let mut segments = Vec::with_capacity(timed.len());
    for index in 0..timed.len() {
        let (start, text) = &timed[index];
        let end = timed
            .get(index + 1)
            .map(|pair| pair.0)
            .unwrap_or_else(|| start.saturating_add(3000));
        if end > 100 * 60 * 60 * 1000 - 1 {
            return Err(Error::input("Final LRC cue exceeds supported duration"));
        }
        segments.push(Segment {
            start: *start,
            end,
            text: text.clone(),
        });
    }
    let rendered = render(&segments);
    parse(&rendered)?;
    Ok(segments)
}

fn render_lrc(segments: &[Segment], job: &Job) -> Result<String> {
    use std::fmt::Write as _;
    let mut out = String::new();
    for segment in segments {
        job.check()?;
        let centiseconds = segment.start / 10;
        let minutes = centiseconds / 6000;
        let seconds = centiseconds / 100 % 60;
        let fraction = centiseconds % 100;
        let text = segment.text.replace(['\n', '\r'], " ");
        writeln!(out, "[{minutes:02}:{seconds:02}.{fraction:02}]{text}")
            .expect("String writes cannot fail");
    }
    Ok(out)
}

fn render(segments: &[Segment]) -> String {
    segments
        .iter()
        .enumerate()
        .map(|(i, s)| {
            format!(
                "{}\n{} --> {}\n{}\n\n",
                i + 1,
                stamp(s.start),
                stamp(s.end),
                s.text
            )
        })
        .collect()
}
fn render_vtt(segments: &[Segment], job: &Job) -> Result<String> {
    use std::fmt::Write as _;
    let mut content = String::from("WEBVTT\n\n");
    for segment in segments {
        job.check()?;
        let start = stamp(segment.start).replace(',', ".");
        let end = stamp(segment.end).replace(',', ".");
        let text = segment
            .text
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;");
        writeln!(content, "{start} --> {end}\n{text}\n").expect("writing to a String cannot fail");
    }
    Ok(content)
}

#[cfg(feature = "asr")]
fn model_bytes(parameter: &str) -> Result<(Vec<u8>, bool)> {
    let private = parameter.trim().is_empty();
    let manifest: serde_json::Value =
        serde_json::from_str(include_str!("../packaging/whisper-model/manifest.json"))?;
    let (path, expected, limit) = if private {
        let id = manifest["id"].as_str().expect("model manifest has an ID");
        let file = manifest["file"]
            .as_str()
            .expect("model manifest has a filename");
        let executable = std::env::current_exe()?.canonicalize()?;
        let path = executable
            .parent()
            .ok_or_else(|| Error::engine("Application directory unavailable"))?
            .join("engines")
            .join(id)
            .join(file);
        let canonical = path
            .canonicalize()
            .map_err(|_| Error::input("Private Whisper Tiny English model pack is missing"))?;
        if canonical != path {
            return Err(Error::input("Private Whisper model path must not redirect"));
        }
        (
            path,
            manifest["sha256"]
                .as_str()
                .expect("model manifest has a SHA-256"),
            manifest["bytes"]
                .as_u64()
                .expect("model manifest has a length"),
        )
    } else {
        let (path, expected) = parameter
            .rsplit_once('|')
            .ok_or_else(|| Error::input("Expected model path | SHA256"))?;
        let path = Path::new(path.trim());
        if !path.is_absolute() {
            return Err(Error::input("Model path must be absolute"));
        }
        (path.to_path_buf(), expected.trim(), 512 * 1024 * 1024)
    };
    if expected.len() != 64 || !expected.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(Error::input("Expected a 64-digit model SHA-256"));
    }
    let bytes = read_bounded(&path, limit)?;
    let actual = format!("{:x}", Sha256::digest(&bytes));
    if !actual.eq_ignore_ascii_case(expected) {
        return Err(Error::input("Model SHA256 mismatch"));
    }
    Ok((bytes, private))
}

#[cfg(feature = "asr")]
fn transcribe(samples: &[f32], parameter: &str, j: &Job) -> Result<Vec<Segment>> {
    use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};
    j.check()?;
    if samples.iter().all(|s| s.abs() < 0.0001) {
        return Err(Error::input("Audio is silent; no transcription attempted"));
    }
    let (bytes, private) = model_bytes(parameter)?;
    j.check()?;
    // Keep native decoder output out of the console and application logs.
    whisper_rs::install_logging_hooks();
    let mut cp = WhisperContextParameters::default();
    cp.use_gpu(false);
    let ctx = WhisperContext::new_from_buffer_with_params(&bytes, cp)
        .map_err(|e| Error::engine(e.to_string()))?;
    let mut state = ctx
        .create_state()
        .map_err(|e| Error::engine(e.to_string()))?;
    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    params.set_n_threads(2);
    params.set_language(Some(if private { "en" } else { "auto" }));
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_timestamps(false);
    params.set_print_special(false);
    params.set_translate(false);
    state
        .full(params, samples)
        .map_err(|e| Error::engine(e.to_string()))?;
    j.check()?;
    let mut out = vec![];
    let count = state
        .full_n_segments()
        .map_err(|e| Error::engine(e.to_string()))?;
    for i in 0..count {
        j.check()?;
        let start = state
            .full_get_segment_t0(i)
            .map_err(|e| Error::engine(e.to_string()))?;
        let end = state
            .full_get_segment_t1(i)
            .map_err(|e| Error::engine(e.to_string()))?;
        let text = state
            .full_get_segment_text(i)
            .map_err(|e| Error::engine(e.to_string()))?
            .trim()
            .to_owned();
        if start >= 0 && end > start && !text.is_empty() {
            out.push(Segment {
                start: start as u64 * 10,
                end: end as u64 * 10,
                text,
            });
        }
    }
    if out.is_empty() {
        return Err(Error::engine("No speech segments returned"));
    }
    Ok(out)
}
pub fn handle(r: Request, j: Job) -> Result<Outcome> {
    j.check()?;
    let mut out = Outcome::default();
    match r.operation.as_str() {
        "prepare-wav" => {
            let frames = prepare_wav(&r.input, &r.output, &j)?;
            out.preview = format!("Prepared {frames} mono samples at 16,000 Hz");
            out.warnings.push("WAV preparation uses weighted averaging for downsampling. High-frequency content can alias.".into());
            out.written = Some(r.output);
        }
        "waveform-svg" => {
            if r.output.extension().and_then(|ext| ext.to_str()) != Some("svg") {
                return Err(Error::input("Expected .svg output"));
            }
            let samples = wav(&r.input, &j)?;
            let svg = waveform_svg(&samples, &j)?;
            j.check()?;
            write_bytes(&r.output, svg.as_bytes())?;
            out.preview = format!(
                "Exported peak waveform for {:.3} seconds",
                samples.len() as f64 / 16_000.0
            );
            out.written = Some(r.output);
        }
        "audio-info" => {
            let s = wav(&r.input, &j)?;
            out.preview = format!(
                "PCM16 mono, 16000 Hz, {} samples, {:.3} seconds",
                s.len(),
                s.len() as f64 / 16000.0
            );
        }
        "detect-voice-activity" => {
            if r.output.extension().and_then(|ext| ext.to_str()) != Some("json") {
                return Err(Error::input("Expected .json output"));
            }
            let samples = wav(&r.input, &j)?;
            let report = analysis::detect_voice_activity(&samples, &j)?;
            j.check()?;
            let json = serde_json::to_vec_pretty(&report)?;
            write_bytes(&r.output, &json)?;
            out.preview = format!(
                "Estimated {} activity intervals across {} ms",
                report.intervals.len(),
                report.duration_ms
            );
            out.warnings.push(report.limitations.into());
            out.written = Some(r.output);
        }
        "analyze-transcript" => {
            if r.output.extension().and_then(|ext| ext.to_str()) != Some("json") {
                return Err(Error::input("Expected .json output"));
            }
            let source = if r.draft.trim().is_empty() {
                String::from_utf8(read_bounded(&r.input, 2 * 1024 * 1024)?).map_err(err)?
            } else {
                r.draft
            };
            let segments = parse(&source)?;
            let report = analysis::analyze_transcript(&segments, &j)?;
            j.check()?;
            let json = serde_json::to_vec_pretty(&report)?;
            write_bytes(&r.output, &json)?;
            out.preview = format!(
                "Analyzed {} words in {} transcript cues",
                report.words, report.cue_count
            );
            out.warnings
                .extend(report.notes.iter().map(|note| (*note).into()));
            out.written = Some(r.output);
        }
        "open-vtt" => {
            let source =
                String::from_utf8(read_bounded(&r.input, 2 * 1024 * 1024)?).map_err(err)?;
            let segments = parse_vtt(&source, &j)?;
            let draft = render(&segments);
            parse(&draft)?;
            out.preview = format!("{} WebVTT cues imported into the SRT draft", segments.len());
            out.draft = Some(draft);
            out.warnings
                .push("Cue identifiers and NOTE blocks are omitted.".into());
        }
        "open-lrc" => {
            let source =
                String::from_utf8(read_bounded(&r.input, 2 * 1024 * 1024)?).map_err(err)?;
            let segments = parse_lrc(&source, &j)?;
            let draft = render(&segments);
            parse(&draft)?;
            out.preview = format!(
                "{} LRC lyric lines imported into the editable draft",
                segments.len()
            );
            out.draft = Some(draft);
            out.warnings.push("LRC has no cue end times; each cue ends at the next timestamp and the final cue is assigned a 3-second duration.".into());
        }
        "replace-text"
        | "shift-timing"
        | "scale-timing"
        | "trim-cues"
        | "split-cue"
        | "merge-cues"
        | "delete-cue"
        | "replace-cue-text"
        | "insert-cue"
        | "set-cue-timing"
        | "shift-cue-range"
        | "remove-duplicate-cues"
        | "normalize-cue-whitespace"
        | "wrap-cue-text"
        | "join-cue-lines"
        | "swap-cue-text"
        | "replace-text-range" => {
            let source = if r.draft.trim().is_empty() {
                String::from_utf8(read_bounded(&r.input, 2 * 1024 * 1024)?).map_err(err)?
            } else {
                r.draft
            };
            let segments = parse(&source)?;
            if r.operation == "shift-timing" {
                out.preview = format!("Shifted {} cue timings", segments.len());
                out.draft = Some(shift_timing(segments, &r.parameter, &j)?);
            } else if r.operation == "replace-text" {
                let (draft, count) = replace_text(segments, &r.parameter, &j)?;
                out.preview = format!("Replaced {count} occurrences in the draft");
                out.draft = Some(draft);
            } else {
                let draft = cue_edit::edit(&r.operation, segments, &r.parameter, &j)?;
                if !r.output.as_os_str().is_empty() {
                    if r.output
                        .extension()
                        .and_then(|extension| extension.to_str())
                        != Some("srt")
                    {
                        return Err(Error::input("Expected .srt output"));
                    }
                    j.check()?;
                    write_bytes(&r.output, draft.as_bytes())?;
                    out.written = Some(r.output);
                }
                out.preview = "Updated subtitle cues".into();
                out.draft = Some(draft);
            }
        }
        "open-json" => {
            let (draft, count) = open_transcript(&r.input, &j)?;
            out.preview = format!("{count} transcript segments imported");
            out.draft = Some(draft);
        }
        "open-srt" | "export-srt" | "export-txt" | "export-vtt" | "export-lrc" | "export-json" => {
            let source = if r.operation == "open-srt" || r.draft.trim().is_empty() {
                String::from_utf8(read_bounded(&r.input, 2 * 1024 * 1024)?).map_err(err)?
            } else {
                r.draft
            };
            let segments = parse(&source)?;
            let count = segments.len();
            if r.operation == "open-srt" {
                out.draft = Some(render(&segments));
            } else {
                let (ext, content) = match r.operation.as_str() {
                    "export-srt" => ("srt", render(&segments)),
                    "export-json" => (
                        "json",
                        serde_json::to_string_pretty(&Transcript {
                            schema_version: 1,
                            segments,
                        })?,
                    ),
                    "export-vtt" => ("vtt", render_vtt(&segments, &j)?),
                    "export-lrc" => ("lrc", render_lrc(&segments, &j)?),
                    _ => (
                        "txt",
                        segments
                            .iter()
                            .map(|s| s.text.as_str())
                            .collect::<Vec<_>>()
                            .join("\n"),
                    ),
                };
                if r.output.extension().and_then(|x| x.to_str()) != Some(ext) {
                    return Err(Error::input(format!("Expected .{ext} output")));
                }
                j.check()?;
                write_bytes(&r.output, content.as_bytes())?;
                if r.operation == "export-lrc" {
                    out.warnings.push("LRC stores cue starts only; cue end times are omitted and multiline text is flattened.".into());
                }
                out.written = Some(r.output);
            }
            out.preview = format!("{count} subtitle segments");
        }
        #[cfg(feature = "asr")]
        "transcribe" => {
            let samples = wav(&r.input, &j)?;
            let segments = transcribe(&samples, &r.parameter, &j)?;
            let s = render(&segments);
            parse(&s)?;
            out.preview = format!(
                "{} experimental segments; review before export",
                segments.len()
            );
            out.draft = Some(s);
            out.warnings.push("No accuracy, silence robustness or language support verified. Cancellation is cooperative before/after native inference, not during it.".into());
        }
        _ => return Err(Error::input("Unknown or unavailable operation")),
    };
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shifts_overlapping_cues_without_changing_text_or_duration() {
        let input =
            "1\n00:00:00,120 --> 00:00:01,250\nCafé\n\n2\n00:00:01,000 --> 00:00:03,000\nTea\n";
        let segments = parse(input).unwrap();
        let shifted = shift_timing(segments.clone(), "-120", &Job::new()).unwrap();
        let actual = parse(&shifted).unwrap();
        assert_eq!((actual[0].start, actual[0].end), (0, 1130));
        assert_eq!((actual[1].start, actual[1].end), (880, 2880));
        assert_eq!(actual[0].text, "Café");
        assert!(shift_timing(segments.clone(), "-121", &Job::new()).is_err());
        assert!(shift_timing(segments, "360000000", &Job::new()).is_err());
    }

    #[test]
    fn waveform_preserves_peaks_and_caps_horizontal_resolution() {
        let mut samples = vec![0.0; 1920];
        samples[0] = 1.0;
        samples[1] = -1.0;
        let svg = waveform_svg(&samples, &Job::new()).unwrap();
        assert!(svg.contains("M0.500 10.000V190.000"));
        assert_eq!(svg.matches("<path ").count(), 961);
        assert!(svg.contains("0.120 s"));
    }

    #[test]
    fn replace_text_preserves_timing_and_rejects_empty_cues() {
        let original = "1\n00:00:00,120 --> 00:00:01,250\nCafé Café\n\n2\n00:00:02,000 --> 00:00:03,000\nTea\n";
        let segments = parse(original).unwrap();
        let (draft, changed) = replace_text(
            segments.clone(),
            r#"{"find":"Café","replace":"Coffee"}"#,
            &Job::new(),
        )
        .unwrap();
        assert_eq!(changed, 2);
        let edited = parse(&draft).unwrap();
        assert_eq!(edited[0].text, "Coffee Coffee");
        assert_eq!((edited[0].start, edited[0].end), (120, 1250));
        assert_eq!(edited[1], segments[1]);
        assert!(replace_text(segments, r#"{"find":"Tea","replace":""}"#, &Job::new()).is_err());
    }

    #[test]
    fn transcript_json_round_trips_draft_text_and_timing() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("transcript.json");
        let source = "1\n00:00:00,250 --> 00:00:01,500\nCafé\nα & β\n";
        handle(
            Request {
                operation: "export-json".into(),
                output: output.clone(),
                draft: source.into(),
                ..Request::default()
            },
            Job::new(),
        )
        .unwrap();
        let json: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&output).unwrap()).unwrap();
        assert_eq!(json["schema_version"], 1);
        assert_eq!(json["segments"][0]["start_ms"], 250);
        assert_eq!(json["segments"][0]["end_ms"], 1500);
        let (draft, count) = open_transcript(&output, &Job::new()).unwrap();
        assert_eq!(count, 1);
        assert_eq!(parse(&draft).unwrap(), parse(source).unwrap());
        let wrong_version = dir.path().join("unsupported.json");
        std::fs::write(&wrong_version, r#"{"schema_version":2,"segments":[]}"#).unwrap();
        assert!(open_transcript(&wrong_version, &Job::new()).is_err());
    }

    #[test]
    fn transcript_analysis_operation_reports_language_and_truthful_estimates() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("analysis.json");
        let draft = "1\n00:00:00,000 --> 00:00:02,000\nThis is a small English transcript. It includes useful words!\n\n2\n00:00:01,500 --> 00:00:03,000\nUseful words appear again.\n";
        let outcome = handle(
            Request {
                operation: "analyze-transcript".into(),
                output: output.clone(),
                draft: draft.into(),
                ..Request::default()
            },
            Job::new(),
        )
        .unwrap();
        assert_eq!(outcome.written, Some(output.clone()));
        let json: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&output).unwrap()).unwrap();
        assert_eq!(json["schema_version"], 1);
        assert_eq!(json["words"], 14);
        assert_eq!(json["cue_count"], 2);
        assert_eq!(json["cue_covered_ms"], 3000);
        assert_eq!(json["language"]["iso_639_3"], "eng");
        assert!(json["asr_confidence"].is_null());
        assert!(handle(
            Request {
                operation: "analyze-transcript".into(),
                output,
                draft: draft.into(),
                ..Request::default()
            },
            Job::new(),
        )
        .is_err());
    }

    #[test]
    fn voice_activity_operation_writes_bounded_energy_report() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("fixture.wav");
        let output = dir.path().join("activity.json");
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 16_000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(&input, spec).unwrap();
        for _ in 0..8_000 {
            writer.write_sample(0i16).unwrap();
        }
        for index in 0..16_000 {
            let sample = (0.2
                * (std::f64::consts::TAU * 220.0 * index as f64 / 16_000.0).sin()
                * f64::from(i16::MAX)) as i16;
            writer.write_sample(sample).unwrap();
        }
        for _ in 0..8_000 {
            writer.write_sample(0i16).unwrap();
        }
        writer.finalize().unwrap();

        let outcome = handle(
            Request {
                operation: "detect-voice-activity".into(),
                input,
                output: output.clone(),
                ..Request::default()
            },
            Job::new(),
        )
        .unwrap();
        assert_eq!(outcome.written, Some(output.clone()));
        let json: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&output).unwrap()).unwrap();
        assert_eq!(json["method"], "adaptive-rms-energy-hysteresis");
        assert_eq!(json["duration_ms"], 2000);
        assert_eq!(json["intervals"].as_array().unwrap().len(), 1);
        assert!((json["speech_active_ms"].as_u64().unwrap() as i64 - 1000).abs() <= 20);
        assert!(json["limitations"]
            .as_str()
            .unwrap()
            .contains("not neural VAD"));
    }

    #[test]
    fn roundtrip() {
        let s =
            "1\n00:00:00,000 --> 00:00:01,500\nCafé\n\n2\n00:00:01,500 --> 00:00:02,000\nWorld\n";
        let a = parse(s).unwrap();
        assert_eq!(parse(&render(&a)).unwrap(), a);
        assert!(parse("1\n00:00:02,000 --> 00:00:01,000\nx").is_err());
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("out.txt");
        handle(
            Request {
                operation: "export-txt".into(),
                draft: s.into(),
                output: p.clone(),
                ..Request::default()
            },
            Job::new(),
        )
        .unwrap();
        assert_eq!(std::fs::read_to_string(p).unwrap(), "Café\nWorld");
    }
    #[test]
    fn imports_plain_webvtt_with_identifiers_and_escaped_text() {
        let source = "WEBVTT\r\n\r\nNOTE comment\r\n\r\ncue-id\r\n00:00.500 --> 00:02.100\r\nCafé &amp; &lt;test&gt;\r\n";
        let cues = parse_vtt(source, &Job::new()).unwrap();
        assert_eq!(
            cues,
            [Segment {
                start: 500,
                end: 2100,
                text: "Café & <test>".into()
            }]
        );
        assert!(parse_vtt(
            "WEBVTT\n\n00:00.000 --> 00:01.000 line:0\nCaption",
            &Job::new()
        )
        .is_err());
    }

    #[test]
    fn webvtt_export_preserves_plain_text_and_timing() {
        let directory = tempfile::tempdir().unwrap();
        let output = directory.path().join("out.vtt");
        handle(
            Request {
                operation: "export-vtt".into(),
                output: output.clone(),
                draft: "1\n00:00:00,500 --> 00:00:02,100\nCafé & <text> --> here".into(),
                ..Request::default()
            },
            Job::new(),
        )
        .unwrap();
        assert_eq!(
            std::fs::read_to_string(output).unwrap(),
            "WEBVTT\n\n00:00:00.500 --> 00:00:02.100\nCafé &amp; &lt;text&gt; --&gt; here\n\n"
        );
    }

    #[test]
    fn lrc_roundtrip_uses_explicit_inferred_duration_and_reports_loss() {
        let source = "[ti:Example]\n[offset:100]\n[00:01.23]First line\n[00:03.00][00:04.00]Repeated lyric\n";
        let segments = parse_lrc(source, &Job::new()).unwrap();
        assert_eq!(
            segments,
            [
                Segment {
                    start: 1330,
                    end: 3100,
                    text: "First line".into()
                },
                Segment {
                    start: 3100,
                    end: 4100,
                    text: "Repeated lyric".into()
                },
                Segment {
                    start: 4100,
                    end: 7100,
                    text: "Repeated lyric".into()
                },
            ]
        );
        let rendered = render_lrc(&segments, &Job::new()).unwrap();
        assert_eq!(
            rendered,
            "[00:01.33]First line\n[00:03.10]Repeated lyric\n[00:04.10]Repeated lyric\n"
        );
        assert!(parse_lrc("[00:01.00]one\n[00:01.00]two", &Job::new()).is_err());
        assert!(parse_lrc("[00:60.00]bad", &Job::new()).is_err());
        let excessive = format!("{}lyrics", "[00:01.00]".repeat(20_001));
        assert!(parse_lrc(&excessive, &Job::new()).is_err());
    }

    #[test]
    fn lrc_export_operation_is_no_overwrite_and_flattens_multiline_text() {
        let directory = tempfile::tempdir().unwrap();
        let output = directory.path().join("out.lrc");
        handle(
            Request {
                operation: "export-lrc".into(),
                output: output.clone(),
                draft: "1\n00:00:01,230 --> 00:00:03,000\nFirst\nsecond".into(),
                ..Request::default()
            },
            Job::new(),
        )
        .unwrap();
        assert_eq!(
            std::fs::read_to_string(&output).unwrap(),
            "[00:01.23]First second\n"
        );
        assert!(handle(
            Request {
                operation: "export-lrc".into(),
                output,
                draft: "1\n00:00:01,230 --> 00:00:03,000\nFirst".into(),
                ..Request::default()
            },
            Job::new()
        )
        .is_err());
    }
    #[test]
    fn prepares_stereo_wav_with_a_partial_final_group() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("input.wav");
        let output = dir.path().join("output.wav");
        let mut writer = hound::WavWriter::create(
            &input,
            hound::WavSpec {
                channels: 2,
                sample_rate: 32_000,
                bits_per_sample: 16,
                sample_format: hound::SampleFormat::Int,
            },
        )
        .unwrap();
        for sample in [1000i16, 3000, -1000, 1000, 2000, 4000] {
            writer.write_sample(sample).unwrap();
        }
        writer.finalize().unwrap();
        assert_eq!(prepare_wav(&input, &output, &Job::new()).unwrap(), 2);
        let mut result = hound::WavReader::open(output).unwrap();
        assert_eq!(result.spec().channels, 1);
        assert_eq!(result.spec().sample_rate, 16_000);
        assert_eq!(
            result
                .samples::<i16>()
                .collect::<std::result::Result<Vec<_>, _>>()
                .unwrap(),
            vec![1000, 3000]
        );
    }

    #[test]
    fn prepares_44_1_khz_using_fractional_frame_overlap() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("in.wav");
        let output = dir.path().join("out.wav");
        let mut writer = hound::WavWriter::create(
            &input,
            hound::WavSpec {
                channels: 1,
                sample_rate: 44_100,
                bits_per_sample: 16,
                sample_format: hound::SampleFormat::Int,
            },
        )
        .unwrap();
        for sample in [0i16, 0, 4410] {
            writer.write_sample(sample).unwrap();
        }
        writer.finalize().unwrap();
        assert_eq!(prepare_wav(&input, &output, &Job::new()).unwrap(), 2);
        let mut result = hound::WavReader::open(output).unwrap();
        let samples = result
            .samples::<i16>()
            .collect::<std::result::Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(samples, [1210, 4410]);
    }

    #[test]
    fn audio() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("x.wav");
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 16000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut w = hound::WavWriter::create(&p, spec).unwrap();
        w.write_sample(16384i16).unwrap();
        w.finalize().unwrap();
        assert_eq!(wav(&p, &Job::new()).unwrap(), vec![0.5]);
    }
}
