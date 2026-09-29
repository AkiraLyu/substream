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
            synthetic: true,
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
        assert!(vtt.starts_with("WEBVTT\n\n1\n01:01:01.001"));
        assert_eq!(timestamp(90_000_000, Format::Srt), "25:00:00,000");
    }

    #[test]
    fn wraps_display_columns_not_utf8_bytes() {
        assert_eq!(wrap("你好世界", 4), "你好\n世界");
        assert_eq!(wrap("one two three", 7), "one two\nthree");
        assert_eq!(wrap("👩‍💻👩‍💻", 2), "👩‍💻\n👩‍💻");
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
