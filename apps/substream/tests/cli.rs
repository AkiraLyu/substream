use std::{
    fs,
    io::Write,
    process::{Command, Stdio},
};

#[test]
fn demo_exports_valid_cues_and_never_overwrites_existing_output() {
    let binary = env!("CARGO_BIN_EXE_substream");
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("captions.vtt");
    assert!(
        Command::new(binary)
            .args(["demo", "--format", "vtt", "--output"])
            .arg(&path)
            .output()
            .unwrap()
            .status
            .success()
    );
    let contents = fs::read_to_string(&path).unwrap();
    assert!(contents.starts_with("WEBVTT\n\n1\n00:00:00.000 --> 00:00:01.000"));
    assert!(contents.contains("00:00:02.000 --> 00:00:02.500"));
    assert!(
        !Command::new(binary)
            .args(["demo", "--output"])
            .arg(&path)
            .output()
            .unwrap()
            .status
            .success()
    );
    assert_eq!(fs::read_to_string(&path).unwrap(), contents);
}

#[test]
fn stdin_eof_preserves_short_audio_and_emits_explicit_demo_provenance() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_substream"))
        .args(["stream", "--backend", "demo"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(&[0; 640]).unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    let events: Vec<serde_json::Value> = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(events[0]["backend"]["synthetic"], true);
    assert_eq!(events[1]["caption"]["end_ms"], 20);
    assert_eq!(events[2]["samples_processed"], 320);
}
