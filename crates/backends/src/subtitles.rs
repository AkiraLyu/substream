//! Import downloaded SRT captions into the common timed-text model.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
    io::Read,
    path::Path,
    time::Duration,
};

use anyhow::{Context, Result, ensure};
use regex::Regex;
use substream_core::transcript::{Segment, Transcript};
use subtp::srt::SubRip;

pub fn read_srt(path: &Path, language: &str) -> Result<Transcript> {
    const MAX_BYTES: u64 = 16 * 1024 * 1024;
    let mut contents = String::new();
    File::open(path)?
        .take(MAX_BYTES + 1)
        .read_to_string(&mut contents)?;
    ensure!(
        contents.len() as u64 <= MAX_BYTES,
        "subtitle file exceeds 16 MiB"
    );
    let captions = SubRip::parse(contents.trim_start_matches('\u{feff}'))
        .context("parse downloaded SRT subtitles")?;
    let markup =
        Regex::new(r"(?i)</?(?:b|i|u|font)(?:\s[^>]*)?>").expect("static subtitle markup pattern");
    let mut cues = Vec::new();
    for caption in captions.subtitles {
        let start: Duration = caption.start.into();
        let end: Duration = caption.end.into();
        let text = caption.text.join("\n");
        let text = markup.replace_all(&text, "");
        let text = html_escape::decode_html_entities(&text).trim().to_owned();
        if text.is_empty() {
            continue;
        }
        let segment = Segment {
            id: cues.len() as u64,
            start_ms: start.as_millis().try_into()?,
            end_ms: end.as_millis().try_into()?,
            text,
        };
        segment.validate()?;
        cues.push(segment);
    }
    ensure!(!cues.is_empty(), "downloaded subtitles contain no text");
    let transcript = Transcript {
        schema_version: 1,
        source: path.display().to_string(),
        language: Some(language.trim_end_matches("-orig").to_owned()),
        segments: combine_overlaps(&cues),
    };
    transcript.validate()?;
    Ok(transcript)
}

/// Preserve the displayed text at each cue boundary, including simultaneous speakers.
/// Identical rolling-caption lines are shown once during their overlapping interval.
fn combine_overlaps(cues: &[Segment]) -> Vec<Segment> {
    let mut events: BTreeMap<u64, Vec<(usize, bool)>> = BTreeMap::new();
    for (index, cue) in cues.iter().enumerate() {
        events.entry(cue.start_ms).or_default().push((index, true));
        events.entry(cue.end_ms).or_default().push((index, false));
    }
    let mut active = BTreeSet::new();
    let mut result: Vec<Segment> = Vec::new();
    let mut events = events.into_iter().peekable();
    while let Some((start_ms, changes)) = events.next() {
        for (index, starts) in changes {
            if starts {
                active.insert(index);
            } else {
                active.remove(&index);
            }
        }
        let Some(&(end_ms, _)) = events.peek() else {
            break;
        };
        if active.is_empty() {
            continue;
        }
        let mut lines = Vec::new();
        for index in &active {
            for line in cues[*index].text.lines() {
                if !lines.contains(&line) {
                    lines.push(line);
                }
            }
        }
        let text = lines.join("\n");
        if let Some(previous) = result.last_mut()
            && previous.end_ms == start_ms
            && previous.text == text
        {
            previous.end_ms = end_ms;
        } else {
            result.push(Segment {
                id: result.len() as u64,
                start_ms,
                end_ms,
                text,
            });
        }
    }
    result
}
