use std::io::{self, Read};

use substream_core::audio::{AudioChunk, FRAME_SAMPLES};

/// Reads raw mono 16 kHz PCM16LE, including short reads and a final partial frame.
pub struct PcmReader<R> {
    reader: R,
    sequence: u32,
    sample: u64,
}

impl<R: Read> PcmReader<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            sequence: 0,
            sample: 0,
        }
    }

    pub fn next_chunk(&mut self) -> io::Result<Option<AudioChunk>> {
        let mut buffer = [0_u8; FRAME_SAMPLES * 2];
        let mut count = 0;
        while count < buffer.len() {
            match self.reader.read(&mut buffer[count..]) {
                Ok(0) => break,
                Ok(n) => count += n,
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e),
            }
        }
        if count == 0 {
            return Ok(None);
        }
        if !count.is_multiple_of(2) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "truncated PCM16 sample",
            ));
        }
        let samples = buffer[..count]
            .as_chunks::<2>()
            .0
            .iter()
            .map(|b| f32::from(i16::from_le_bytes([b[0], b[1]])) / 32768.0)
            .collect();
        let chunk = AudioChunk::new(self.sequence, self.sample, samples)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        self.sample = chunk.end_sample();
        self.sequence = self.sequence.wrapping_add(1);
        Ok(Some(chunk))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct ByteReader<'a>(&'a [u8]);
    impl Read for ByteReader<'_> {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            self.0.read(&mut buffer[..1])
        }
    }

    #[test]
    fn reassembles_short_reads_without_losing_tail_samples() {
        let mut reader = PcmReader::new(ByteReader(&[0, 128, 255, 127]));
        let mut samples = Vec::new();
        while let Some(chunk) = reader.next_chunk().unwrap() {
            assert_eq!(chunk.start_sample(), samples.len() as u64);
            samples.extend_from_slice(chunk.samples());
        }
        assert_eq!(samples, [-1.0, 32767.0 / 32768.0]);
        assert!(PcmReader::new(&[0_u8][..]).next_chunk().is_err());
    }
}
