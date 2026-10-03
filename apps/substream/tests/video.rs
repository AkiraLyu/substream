/// Exercise the executable against local media with real yt-dlp and FFmpeg.
#[test]
#[ignore = "requires yt-dlp, ffmpeg and python3 on PATH"]
fn browser_cookies_download_the_complete_video_and_export_subtitles() {
    let output = std::process::Command::new("python3")
        .arg(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/video_flow.py"))
        .arg(env!("CARGO_BIN_EXE_substream"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
