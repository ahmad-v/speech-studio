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

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CueText {
    cue: usize,
    text: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct InsertCue {
    index: usize,
    start_ms: u64,
    end_ms: u64,
    text: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CueTiming {
    cue: usize,
    start_ms: u64,
    end_ms: u64,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CueRangeShift {
    first: usize,
    last: usize,
    offset_ms: i64,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WrapText {
    max_graphemes: usize,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CuePair {
    first: usize,
    second: usize,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CueRangeReplace {
    first: usize,
    last: usize,
    find: String,
    replace: String,
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
        "replace-cue-text" => {
            let edit: CueText = serde_json::from_str(parameter)?;
            let index = cue_index(edit.cue as u64, cues.len())?;
            cues[index].text = edit.text;
        }
        "insert-cue" => {
            let insert: InsertCue = serde_json::from_str(parameter)?;
            if cues.len() >= 20_000 || insert.index == 0 || insert.index > cues.len() + 1 {
                return Err(Error::input(
                    "Cue insertion index is out of range or cue limit reached",
                ));
            }
            if insert.start_ms >= insert.end_ms || insert.end_ms > MAX_TIMESTAMP {
                return Err(Error::input("Inserted cue needs valid millisecond timing"));
            }
            let index = insert.index - 1;
            if index > 0 && cues[index - 1].start > insert.start_ms
                || index < cues.len() && cues[index].start < insert.start_ms
            {
                return Err(Error::input("Inserted cue must preserve start-time order"));
            }
            cues.insert(
                index,
                Segment {
                    start: insert.start_ms,
                    end: insert.end_ms,
                    text: insert.text,
                },
            );
        }
        "set-cue-timing" => {
            let timing: CueTiming = serde_json::from_str(parameter)?;
            let index = cue_index(timing.cue as u64, cues.len())?;
            if timing.start_ms >= timing.end_ms || timing.end_ms > MAX_TIMESTAMP {
                return Err(Error::input("Cue needs valid millisecond timing"));
            }
            cues[index].start = timing.start_ms;
            cues[index].end = timing.end_ms;
        }
        "shift-cue-range" => {
            let range: CueRangeShift = serde_json::from_str(parameter)?;
            let first = cue_index(range.first as u64, cues.len())?;
            let last = cue_index(range.last as u64, cues.len())?;
            if first > last {
                return Err(Error::input("Cue range must be in ascending order"));
            }
            for cue in &mut cues[first..=last] {
                job.check()?;
                cue.start = shift_time(cue.start, range.offset_ms)?;
                cue.end = shift_time(cue.end, range.offset_ms)?;
            }
        }
        "remove-duplicate-cues" => {
            let mut retained = Vec::with_capacity(cues.len());
            for cue in cues {
                job.check()?;
                if !retained.iter().any(|previous: &Segment| previous == &cue) {
                    retained.push(cue);
                }
            }
            cues = retained;
        }
        "normalize-cue-whitespace" => {
            for cue in &mut cues {
                job.check()?;
                cue.text = cue
                    .text
                    .lines()
                    .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
                    .filter(|line| !line.is_empty())
                    .collect::<Vec<_>>()
                    .join("\n");
            }
        }
        "wrap-cue-text" => {
            let settings: WrapText = serde_json::from_str(parameter)?;
            if !(10..=200).contains(&settings.max_graphemes) {
                return Err(Error::input("Wrap width must be 10 to 200 graphemes"));
            }
            use unicode_segmentation::UnicodeSegmentation;
            for cue in &mut cues {
                job.check()?;
                let mut lines = Vec::new();
                let mut line = String::new();
                let mut count = 0;
                for word in cue.text.split_whitespace() {
                    job.check()?;
                    let word_count = word.graphemes(true).count();
                    let required = word_count + if line.is_empty() { 0 } else { 1 };
                    if !line.is_empty() && count + required > settings.max_graphemes {
                        lines.push(std::mem::take(&mut line));
                        count = 0;
                    }
                    if !line.is_empty() {
                        line.push(' ');
                        count += 1;
                    }
                    line.push_str(word);
                    count += word_count;
                }
                if !line.is_empty() {
                    lines.push(line);
                }
                cue.text = lines.join("\n");
            }
        }
        "join-cue-lines" => {
            for cue in &mut cues {
                job.check()?;
                cue.text = cue.text.split_whitespace().collect::<Vec<_>>().join(" ");
            }
        }
        "swap-cue-text" => {
            let pair: CuePair = serde_json::from_str(parameter)?;
            let first = cue_index(pair.first as u64, cues.len())?;
            let second = cue_index(pair.second as u64, cues.len())?;
            if first != second {
                job.check()?;
                let (a, b) = if first < second {
                    let (left, right) = cues.split_at_mut(second);
                    (&mut left[first].text, &mut right[0].text)
                } else {
                    let (left, right) = cues.split_at_mut(first);
                    (&mut right[0].text, &mut left[second].text)
                };
                std::mem::swap(a, b);
            }
        }
        "replace-text-range" => {
            let edit: CueRangeReplace = serde_json::from_str(parameter)?;
            let first = cue_index(edit.first as u64, cues.len())?;
            let last = cue_index(edit.last as u64, cues.len())?;
            if first > last || edit.find.is_empty() {
                return Err(Error::input(
                    "Choose an ascending cue range and nonempty find text",
                ));
            }
            let mut total = render(&cues).len();
            for cue in &mut cues[first..=last] {
                job.check()?;
                let count = cue.text.matches(&edit.find).count();
                let old_len = cue.text.len();
                let removed = edit
                    .find
                    .len()
                    .checked_mul(count)
                    .ok_or_else(|| Error::input("Replacement size overflow"))?;
                let added = edit
                    .replace
                    .len()
                    .checked_mul(count)
                    .ok_or_else(|| Error::input("Replacement size overflow"))?;
                let new_len = old_len
                    .checked_sub(removed)
                    .and_then(|n| n.checked_add(added))
                    .ok_or_else(|| Error::input("Replacement size overflow"))?;
                total = total
                    .checked_sub(old_len)
                    .and_then(|n| n.checked_add(new_len))
                    .filter(|n| *n <= 2 * 1024 * 1024)
                    .ok_or_else(|| Error::input("Edited transcript exceeds the 2 MiB limit"))?;
                cue.text = cue.text.replace(&edit.find, &edit.replace);
            }
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

fn shift_time(time: u64, offset: i64) -> Result<u64> {
    let shifted = i128::from(time) + i128::from(offset);
    if !(0..=i128::from(MAX_TIMESTAMP)).contains(&shifted) {
        return Err(Error::input(
            "Shifted cue is outside the SRT timestamp range",
        ));
    }
    Ok(shifted as u64)
}
