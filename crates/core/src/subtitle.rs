use std::{fmt::Write, num::NonZeroUsize};

use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::{Result, transcript::Transcript};

#[derive(Debug, Clone, Copy)]
pub enum Format {
    Srt,
    Vtt,
}

/// Preserve ASR segment times. Word-timed resegmentation belongs in a separate
/// stage; splitting a long cue into invented equal-duration cues is incorrect.
pub fn render(transcript: &Transcript, format: Format, columns: NonZeroUsize) -> Result<String> {
    transcript.validate()?;
    let mut output = String::new();
    if matches!(format, Format::Vtt) {
        output.push_str("WEBVTT\n\n");
    }
    for (index, segment) in transcript.segments.iter().enumerate() {
        let text = wrap(&segment.text, columns.get());
        let text = text
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;");
        writeln!(
            output,
            "{}\n{} --> {}\n{}\n",
            index + 1,
            timestamp(segment.start_ms, format),
            timestamp(segment.end_ms, format),
            text
        )
        .expect("writing into a String cannot fail");
    }
    Ok(output)
}

fn timestamp(ms: u64, format: Format) -> String {
    let separator = match format {
        Format::Srt => ',',
        Format::Vtt => '.',
    };
    format!(
        "{:02}:{:02}:{:02}{separator}{:03}",
        ms / 3_600_000,
        ms / 60_000 % 60,
        ms / 1_000 % 60,
        ms % 1_000
    )
}

fn wrap(text: &str, columns: usize) -> String {
    let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut output = String::new();
    let mut width = 0;
    for word in normalized.split_word_bounds() {
        let word_width = word.width();
        if width > 0 && width + word_width > columns {
            while output.ends_with(' ') {
                output.pop();
            }
            output.push('\n');
            width = 0;
        }
        for grapheme in word.graphemes(true) {
            if width == 0 && grapheme == " " {
                continue;
            }
            let next_width = grapheme.width();
            if width > 0 && width + next_width > columns {
                output.push('\n');
                width = 0;
            }
            output.push_str(grapheme);
            width += next_width;
        }
    }
    output.trim().to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transcript::Segment;

    fn transcript() -> Transcript {
        Transcript {
            schema_version: 1,
            source: "test".into(),
            language: None,
            segments: vec![Segment {
                id: 0,
                start_ms: 3_661_001,
                end_ms: 3_662_050,
                text: "你好世界 <tag> &\n\ntext".into(),
            }],
        }
    }

    #[test]
    fn formats_timestamps_and_escapes_markup_without_forging_cues() {
        let srt = render(&transcript(), Format::Srt, NonZeroUsize::new(42).unwrap()).unwrap();
        assert!(srt.contains("01:01:01,001 --> 01:01:02,050"));
        assert!(srt.contains("&lt;tag&gt; &amp; text"));
        let vtt = render(&transcript(), Format::Vtt, NonZeroUsize::new(42).unwrap()).unwrap();
        assert!(vtt.starts_with("WEBVTT\n"));
        assert!(vtt.contains("01:01:01.001 --> 01:01:02.050"));
        assert!(vtt.contains("&lt;tag&gt; &amp; text"));
    }

    #[test]
    fn exported_lines_fit_the_width_without_losing_text_or_splitting_characters() {
        let text = "你好世界 one two 👩‍💻 café e\u{301}";
        let mut transcript = transcript();
        transcript.segments[0].text = text.into();
        let output = render(&transcript, Format::Srt, NonZeroUsize::new(8).unwrap()).unwrap();
        let lines: Vec<_> = output
            .lines()
            .skip(2)
            .filter(|line| !line.is_empty())
            .collect();
        assert!(lines.iter().all(|line| line.width() <= 8));

        let visible_characters = |value: &str| {
            value
                .graphemes(true)
                .filter(|character| !character.chars().all(char::is_whitespace))
                .map(str::to_owned)
                .collect::<Vec<_>>()
        };
        let actual: Vec<_> = lines
            .iter()
            .flat_map(|line| visible_characters(line))
            .collect();
        assert_eq!(actual, visible_characters(text));
    }

    #[test]
    fn rejects_overlaps_instead_of_silently_retiming() {
        let mut transcript = transcript();
        transcript.segments.push(Segment {
            id: 1,
            start_ms: 3_661_001,
            end_ms: 3_663_000,
            text: "bad".into(),
        });
        assert!(render(&transcript, Format::Srt, NonZeroUsize::new(42).unwrap()).is_err());
    }
}
