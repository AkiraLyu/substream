use std::{fs, io::Write, os::unix::fs::PermissionsExt, process::Command};

/// Real FFmpeg normalization + a contract fixture standing in for whisper-cli.
/// This checks process boundaries, not speech recognition quality.
#[test]
#[ignore = "requires ffmpeg and python3 on PATH"]
fn local_media_reaches_the_batch_adapter_and_exports_valid_subtitles() {
    let directory = tempfile::tempdir().unwrap();
    let input = directory.path().join("input with spaces $(literal).wav");
    let model = directory.path().join("model.bin");
    let backend = directory.path().join("fixture-whisper");
    fs::write(&model, b"fixture, not a model").unwrap();
    write_stereo_wav(&input);
    fs::write(
        &backend,
        r#"#!/usr/bin/env python3
import json
import pathlib
import sys
import wave
args = sys.argv[1:]
def option(name):
    return args[args.index(name) + 1]
assert pathlib.Path(option('--model')).is_file()
assert option('--language') == 'auto'
assert '--output-json' in args
with wave.open(option('--file')) as audio:
    assert audio.getnchannels() == 1
    assert audio.getframerate() == 16000
    assert audio.getsampwidth() == 2
    assert audio.getnframes() == 64000
output = {'result': {'language': 'en'}, 'transcription': [
    {'offsets': {'from': 0, 'to': 1230}, 'text': 'A local subtitle.'}]}
pathlib.Path(option('--output-file') + '.json').write_text(json.dumps(output))
"#,
    )
    .unwrap();
    fs::set_permissions(&backend, fs::Permissions::from_mode(0o755)).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_substream"))
        .arg("transcribe")
        .arg(input)
        .arg("--model")
        .arg(model)
        .arg("--whisper-bin")
        .arg(backend)
        .args(["--format", "srt"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "1\n00:00:00,000 --> 00:00:01,230\nA local subtitle.\n\n"
    );
}

fn write_stereo_wav(path: &std::path::Path) {
    let data_bytes = 44_100_u32 * 4 * 2 * 2;
    let mut file = fs::File::create(path).unwrap();
    file.write_all(b"RIFF").unwrap();
    file.write_all(&(36 + data_bytes).to_le_bytes()).unwrap();
    file.write_all(b"WAVEfmt ").unwrap();
    file.write_all(&16_u32.to_le_bytes()).unwrap();
    file.write_all(&1_u16.to_le_bytes()).unwrap();
    file.write_all(&2_u16.to_le_bytes()).unwrap();
    file.write_all(&44_100_u32.to_le_bytes()).unwrap();
    file.write_all(&(44_100_u32 * 4).to_le_bytes()).unwrap();
    file.write_all(&4_u16.to_le_bytes()).unwrap();
    file.write_all(&16_u16.to_le_bytes()).unwrap();
    file.write_all(b"data").unwrap();
    file.write_all(&data_bytes.to_le_bytes()).unwrap();
    file.write_all(&vec![0; data_bytes as usize]).unwrap();
}
