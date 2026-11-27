use super::{parse, render, Segment};
use localwright_core::{Error, Job, Result};
use serde::Deserialize;

const MAX_TIMESTAMP: u64 = 100 * 60 * 60 * 1000 - 1;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Split {
    cue: usize,
    at_ms: u64,
    first: String,
    second: String,
}

pub(super) fn edit(
    operation: &str,
    mut cues: Vec<Segment>,
    parameter: &str,
    job: &Job,
) -> Result<String> {
    if parameter.len() > 128 * 1024 {
        return Err(Error::input("Cue edit settings exceed 128 KiB"));
    }
    match operation {
        "scale-timing" => {
            let percent = number(parameter)?;
            if !(1..=1000).contains(&percent) {
                return Err(Error::input("Enter a whole percentage from 1 to 1000"));
            }
            for cue in &mut cues {
                job.check()?;
                cue.start = scaled(cue.start, percent)?;
                cue.end = scaled(cue.end, percent)?;
            }
        }
        "trim-cues" => {
            let (start, end) = pair(parameter)?;
            if start >= end || end > MAX_TIMESTAMP {
                return Err(Error::input("Invalid millisecond time range"));
            }
            let mut trimmed = Vec::new();
            for mut cue in cues {
                job.check()?;
                cue.start = cue.start.max(start);
                cue.end = cue.end.min(end);
                if cue.start < cue.end {
                    trimmed.push(cue);
                }
            }
            cues = trimmed;
        }
        "split-cue" => {
            let split: Split = serde_json::from_str(parameter)?;
            let position = cue_index(split.cue as u64, cues.len())?;
            let cue = &mut cues[position];
            if !(cue.start < split.at_ms && split.at_ms < cue.end) {
                return Err(Error::input("Split time must be strictly inside the cue"));
            }
            let second = Segment {
                start: split.at_ms,
                end: cue.end,
                text: split.second,
            };
            cue.end = split.at_ms;
            cue.text = split.first;
            let order = cues.partition_point(|cue| cue.start <= split.at_ms);
            cues.insert(order, second);
        }
        "merge-cues" => {
            let (first, last) = pair(parameter)?;
            let start = cue_index(first, cues.len())?;
            let end = cue_index(last, cues.len())?;
            if start >= end {
                return Err(Error::input("Select at least two consecutive cues"));
            }
            let merged = &cues[start..=end];
            let mut text = String::new();
            let mut end_ms = 0;
            for cue in merged {
                job.check()?;
                if !text.is_empty() {
                    text.push('\n');
                }
                text.push_str(&cue.text);
                end_ms = end_ms.max(cue.end);
            }
            let replacement = Segment {
                start: cues[start].start,
                end: end_ms,
                text,
            };
            cues.splice(start..=end, [replacement]);
        }
        "delete-cue" => {
            let position = cue_index(number(parameter)?, cues.len())?;
            cues.remove(position);
        }
        _ => return Err(Error::input("Unknown cue edit")),
    }
    job.check()?;
    let draft = render(&cues);
    if parse(&draft)? != cues {
        return Err(Error::input(
            "Edited cues cannot be represented in the supported SRT format",
        ));
    }
    Ok(draft)
}

fn number(text: &str) -> Result<u64> {
    text.trim()
        .parse()
        .map_err(|_| Error::input("Expected a nonnegative whole number"))
}

fn pair(text: &str) -> Result<(u64, u64)> {
    let (first, last) = text
        .split_once(',')
        .ok_or_else(|| Error::input("Enter two comma-separated numbers"))?;
    Ok((number(first)?, number(last)?))
}

fn cue_index(number: u64, len: usize) -> Result<usize> {
    if number == 0 || number > len as u64 {
        return Err(Error::input("Cue number is out of range"));
    }
    Ok(number as usize - 1)
}

fn scaled(time: u64, percent: u64) -> Result<u64> {
    let time = (time * percent + 50) / 100;
    if time > MAX_TIMESTAMP {
        return Err(Error::input(
            "Scaled timing exceeds the SRT timestamp range",
        ));
    }
    Ok(time)
}
