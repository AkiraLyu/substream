use std::num::NonZeroUsize;

use unicode_segmentation::UnicodeSegmentation;

/// Stability is a display hint, never an irreversible commitment. A later
/// hypothesis (especially a final) may revise an earlier stable prefix.
pub struct Stabilizer {
    observations: NonZeroUsize,
    previous: Vec<String>,
    counts: Vec<usize>,
}

impl Stabilizer {
    pub fn new(observations: NonZeroUsize) -> Self {
        Self {
            observations,
            previous: Vec::new(),
            counts: Vec::new(),
        }
    }

    pub fn update(&mut self, text: &str, is_final: bool) -> (String, String) {
        let current: Vec<String> = text.graphemes(true).map(str::to_owned).collect();
        let common = current
            .iter()
            .zip(&self.previous)
            .take_while(|(a, b)| a == b)
            .count();
        let counts: Vec<usize> = (0..current.len())
            .map(|i| {
                if i < common {
                    self.counts[i].saturating_add(1)
                } else {
                    1
                }
            })
            .collect();
        let stable = if is_final {
            current.len()
        } else {
            counts
                .iter()
                .take_while(|n| **n >= self.observations.get())
                .count()
        };
        let result = (current[..stable].concat(), current[stable..].concat());
        self.previous = current;
        self.counts = counts;
        result
    }

    pub fn reset(&mut self) {
        self.previous.clear();
        self.counts.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multilingual_prefixes_use_graphemes_and_allow_corrections() {
        let mut stabilizer = Stabilizer::new(NonZeroUsize::new(2).unwrap());
        assert_eq!(
            stabilizer.update("你好👩‍💻", false),
            ("".into(), "你好👩‍💻".into())
        );
        assert_eq!(
            stabilizer.update("你好👩‍💻！", false),
            ("你好👩‍💻".into(), "！".into())
        );
        assert_eq!(
            stabilizer.update("你们好", false),
            ("你".into(), "们好".into())
        );
        assert_eq!(
            stabilizer.update("你们好。", true),
            ("你们好。".into(), "".into())
        );
        stabilizer.reset();
        assert_eq!(stabilizer.update("你", false).0, "");
    }
}
