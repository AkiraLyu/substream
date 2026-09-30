use std::{
    fs,
    io::Write,
    process::{Command, Stdio},
};
use substream_protocol::ServerMessage;

#[test]
fn refuses_to_overwrite_existing_subtitles() {
    let binary = env!("CARGO_BIN_EXE_substream");
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("captions.vtt");
    let contents = "Existing subtitles edited by the user.\n";
    fs::write(&path, contents).unwrap();
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
    let events: Vec<ServerMessage> = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert!(
        events.iter().any(
            |event| matches!(event, ServerMessage::Ready { backend, .. } if backend.synthetic)
        )
    );
    assert!(events.iter().any(|event| matches!(event, ServerMessage::Caption { caption } if caption.is_final && !caption.segment().text.is_empty())));
    assert!(matches!(
        events.last(),
        Some(ServerMessage::Finished {
            samples_processed: 320,
            ..
        })
    ));
}
