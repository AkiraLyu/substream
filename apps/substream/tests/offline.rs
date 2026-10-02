use std::{fs, io::Write, os::unix::fs::PermissionsExt, process::Command};

/// Convert real media with FFmpeg and export a known recognition result.
/// The substitute recognizer keeps this test independent of model accuracy.
#[test]
#[ignore = "requires ffmpeg and python3 on PATH"]
fn local_media_reaches_the_batch_adapter_and_exports_valid_subtitles() {
    let directory = tempfile::tempdir().unwrap();
    let input = directory.path().join("input with spaces $(literal).wav");
    let model = directory.path().join("model.bin");
    let backend = directory.path().join("fixture-whisper");
    let subtitles = directory.path().join("captions.srt");
    fs::write(&model, b"fixture, not a model").unwrap();
    fs::write(
        directory.path().join("transcript.json"),
        include_bytes!("../../../fixtures/whisper-output.json"),
    )
    .unwrap();
    write_stereo_wav(&input);
    fs::write(
        &backend,
        r#"#!/usr/bin/env python3
import pathlib
import sys
import wave
args = sys.argv[1:]
def option(*names):
    return next(args[i + 1] for i, arg in enumerate(args) if arg in names)
with wave.open(option('--file', '-f')) as audio:
    assert audio.getnchannels() == 1
    assert audio.getframerate() == 16000
    assert audio.getsampwidth() == 2
    assert audio.getnframes() == 64000
result = pathlib.Path(__file__).with_name('transcript.json').read_bytes()
pathlib.Path(option('--output-file', '-of') + '.json').write_bytes(result)
"#,
    )
    .unwrap();
    fs::set_permissions(&backend, fs::Permissions::from_mode(0o755)).unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_substream"));
    command
        .arg("transcribe")
        .arg(input)
        .arg("--model")
        .arg(model)
        .arg("--whisper-bin")
        .arg(backend)
        .args(["--format", "srt", "--output"])
        .arg(&subtitles);
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let contents = fs::read_to_string(&subtitles).unwrap();
    assert!(contents.contains("00:00:00,000 --> 00:00:01,230"));
    assert!(contents.contains("こんにちは。"));
    assert!(contents.contains("00:00:01,500 --> 00:00:03,100"));
    assert!(contents.contains("これは字幕のテストです。"));
    assert!(!command.output().unwrap().status.success());
    assert_eq!(fs::read_to_string(subtitles).unwrap(), contents);
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
