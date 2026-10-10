"""Local-only download test; the recognizer checks complete audio, not model accuracy."""

import http.server
import json
import os
from pathlib import Path
import signal
import socket
import sqlite3
import subprocess
import sys
import tempfile
import threading
import time
import urllib.error
import urllib.request


def run(*args):
    return subprocess.run(args, check=True, capture_output=True, text=True, timeout=60)


def check_result(result, url):
    directory = Path(result["directory"])
    assert directory.stat().st_mode & 0o077 == 0
    assert Path(result["video"]).stat().st_size > 0
    srt = Path(result["srt"]).read_text()
    vtt = Path(result["vtt"]).read_text()
    assert "00:00:01,500 --> 00:00:03,100" in srt
    assert "これは字幕のテストです。" in srt
    assert vtt.startswith("WEBVTT\n")
    assert "00:00:01.500 --> 00:00:03.100" in vtt
    document = json.loads(Path(result["document"]).read_text())
    assert document["source"]["url"] == url
    assert document["source"]["title"]
    assert document["transcript"]["language"] == "ja"
    assert document["transcript"]["segments"][-1]["end_ms"] == 3100
    assert not list(directory.glob("*.wav")), "temporary audio must be removed"
    return document


def exercise(root, binary):
    media = root / "clip.mkv"
    run("ffmpeg", "-hide_banner", "-loglevel", "error", "-f", "lavfi", "-i",
        "color=size=32x32:rate=5", "-f", "lavfi", "-i", "anullsrc=r=48000:cl=stereo",
        "-t", "4", "-c:v", "ffv1", "-c:a", "pcm_s16le", str(media))
    content = media.read_bytes()
    captions = ('WEBVTT\n\n00:00:00.500 --> 00:00:02.000\n'
                '<i>已有字幕</i> &amp; text\n\n'
                '00:00:01.500 --> 00:00:03.000\n另一个说话人\n').encode()
    page = (b'<html><title>Captioned video</title><video controls src="/clip.mkv">'
            b'<track kind="subtitles" src="/captions.vtt" srclang="zh" label="Chinese">'
            b'</video></html>')

    class MediaHandler(http.server.BaseHTTPRequestHandler):
        def do_POST(self):
            assert self.path == "/v1/chat/completions"
            assert self.headers.get("Authorization") == "Bearer summary-fixture"
            assert self.headers.get("Cookie") is None
            body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
            assert body["model"] == "fixture-model"
            assert "これは字幕のテストです。" in body["messages"][1]["content"]
            assert body["messages"][0]["content"] == "Write brief notes."
            response = json.dumps({"choices": [{"finish_reason": "stop",
                "message": {"content": "# Notes\n\nVideo summary at [00:00:01]."}}]}).encode()
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(response)))
            self.end_headers()
            self.wfile.write(response)

        def do_GET(self):
            if self.headers.get("Cookie") != "session=fixture-login":
                self.send_error(403)
                return
            body, mime = {
                "/watch": (page, "text/html"),
                "/captions.vtt": (captions, "text/vtt"),
            }.get(self.path, (content, "video/x-matroska"))
            self.send_response(200)
            self.send_header("Content-Type", mime)
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)

        def log_message(self, *_args):
            pass

    media_server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), MediaHandler)
    threading.Thread(target=media_server.serve_forever, daemon=True).start()
    url = f"http://127.0.0.1:{media_server.server_port}/clip.mkv"
    profile = root / "firefox profile"
    profile.mkdir()
    with sqlite3.connect(profile / "cookies.sqlite") as db:
        db.execute("CREATE TABLE moz_cookies (host, name, value, path, expiry, isSecure)")
        db.execute("INSERT INTO moz_cookies VALUES (?, ?, ?, ?, ?, ?)",
                   ("127.0.0.1", "session", "fixture-login", "/", int(time.time()) + 3600, 0))

    (root / "model.bin").write_bytes(b"fixture, not a model")
    fixture = Path(__file__).resolve().parents[3] / "fixtures/whisper-output.json"
    (root / "transcript.json").write_bytes(fixture.read_bytes())
    recognizer = root / "fixture-whisper"
    recognizer.write_text('''#!/usr/bin/env python3
import pathlib, sys, wave
args = sys.argv[1:]
def option(name):
    return args[args.index(name) + 1]
with wave.open(option('--file')) as audio:
    assert (audio.getnchannels(), audio.getframerate(), audio.getsampwidth()) == (1, 16000, 2)
    assert audio.getnframes() == 64000, "the entire four-second clip must reach recognition"
result = pathlib.Path(__file__).with_name('transcript.json').read_bytes()
pathlib.Path(option('--output-file') + '.json').write_bytes(result)
''')
    recognizer.chmod(0o755)
    config = root / "video.toml"
    config.write_text(
        'output_dir = "output"\nmodel = "model.bin"\nwhisper_bin = "./fixture-whisper"\n'
        f'cookies_from_browser = {json.dumps("firefox:" + str(profile))}\n'
        'timeout_secs = 30\n'
    )
    summary_config = root / "summary.json"
    defaults = json.loads((Path(__file__).resolve().parents[3] / "configs/summary.example.json").read_text())
    defaults.update(endpoint=f"http://127.0.0.1:{media_server.server_port}/v1/chat/completions",
                    api_key="summary-fixture", model="fixture-model", system_prompt="Write brief notes.")
    summary_config.write_text(json.dumps(defaults))
    token_file = root / "token"
    run(binary, "token", "--output", str(token_file))
    token = token_file.read_text().strip()
    origin = "chrome-extension://test"
    with socket.socket() as reservation:
        reservation.bind(("127.0.0.1", 0))
        address = f"127.0.0.1:{reservation.getsockname()[1]}"
    service = subprocess.Popen(
        [binary, "serve", "--listen", address, "--video-config", str(config),
         "--token-file", str(token_file), "--allow-origin", origin,
         "--summary-config", str(summary_config)],
        stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, text=True,
        env={**os.environ, "RUST_LOG": "info", "NO_COLOR": "1"},
    )
    try:
        deadline = time.monotonic() + 5
        while True:
            assert service.poll() is None, service.stderr.read()
            try:
                with urllib.request.urlopen(f"http://{address}/health", timeout=1):
                    break
            except urllib.error.URLError:
                assert time.monotonic() < deadline, "service did not start"
                time.sleep(0.05)

        def request(method, path, body=None):
            data = None if body is None else json.dumps(body).encode()
            request = urllib.request.Request(f"http://{address}{path}", data, method=method,
                headers={"Authorization": f"Bearer {token}", "Origin": origin,
                         "Content-Type": "application/json"})
            with urllib.request.urlopen(request, timeout=10) as response:
                return json.load(response)

        job = request("POST", "/v1/video/jobs", {"url": url})
        path = f'/v1/video/jobs/{job["id"]}'
        deadline = time.monotonic() + 45
        while job["stage"] not in ("completed", "failed", "cancelled"):
            assert time.monotonic() < deadline, "video job timed out"
            time.sleep(0.05)
            job = request("GET", path)
        assert job["stage"] == "completed", job
        document = check_result(job["result"], url)
        assert request("GET", path + "/document") == document
        summary = request("POST", path + "/summary", {})
        assert summary["markdown"] == "# Notes\n\nVideo summary at [00:00:01]."
        assert summary["source"] == document["source"]
        markdown = root / "summary.md"
        run(binary, "summarize", job["result"]["document"], "--config", str(summary_config),
            "--output", str(markdown))
        assert markdown.read_text() == summary["markdown"]

        # CLI uses the same result contract and preserves previous output on a repeated request.
        output = run(binary, "video", url, "--config", str(config))
        cli_job = json.loads(output.stdout.splitlines()[-1])
        assert cli_job["stage"] == "completed", cli_job
        assert cli_job["result"]["directory"] != job["result"]["directory"]
        assert check_result(cli_job["result"], url) == document
        assert check_result(job["result"], url) == document

        # Existing captions work without a model or recognizer. File cookies override
        # the configured browser, and the user's cookie jar remains unchanged.
        cookie_file = root / "cookies.txt"
        cookie_file.write_text("# Netscape HTTP Cookie File\n"
            "127.0.0.1\tFALSE\t/\tFALSE\t0\tsession\tfixture-login\n")
        original_cookies = cookie_file.read_bytes()
        caption_config = root / "captions.toml"
        caption_config.write_text('output_dir = "output"\nlanguage = "zh"\n'
            'cookies_from_browser = "firefox:/missing/profile"\n')
        caption_url = url.replace("/clip.mkv", "/watch")
        output = run(binary, "video", caption_url, "--config", str(caption_config),
                     "--cookies-file", str(cookie_file))
        caption_job = json.loads(output.stdout.splitlines()[-1])
        assert caption_job["stage"] == "completed", caption_job
        result = caption_job["result"]
        assert result["subtitle_source"] == "provided"
        assert Path(result["video"]).stat().st_size > 0
        imported = json.loads(Path(result["document"]).read_text())
        assert imported["subtitle_source"] == "provided"
        assert imported["transcript"]["language"] == "zh"
        segments = imported["transcript"]["segments"]
        assert segments == [
            {"id": 0, "start_ms": 500, "end_ms": 1500, "text": "已有字幕 & text"},
            {"id": 1, "start_ms": 1500, "end_ms": 2000,
             "text": "已有字幕 & text\n另一个说话人"},
            {"id": 2, "start_ms": 2000, "end_ms": 3000, "text": "另一个说话人"},
        ], segments
        assert "&amp; text" in Path(result["srt"]).read_text()
        assert Path(result["vtt"]).read_text().startswith("WEBVTT\n")
        assert not (Path(result["directory"]) / "cookies.txt").exists()
        assert cookie_file.read_bytes() == original_cookies

        anonymous = subprocess.run([binary, "video", caption_url, "--config", str(config),
                                    "--no-cookies"], capture_output=True, text=True, timeout=30)
        assert anonymous.returncode != 0, "anonymous download must not use configured cookies"
        assert "403" in anonymous.stderr
    finally:
        service.send_signal(signal.SIGTERM)
        try:
            service.communicate(timeout=5)
        except subprocess.TimeoutExpired:
            service.kill()
            service.communicate()
            raise
        media_server.shutdown()
        media_server.server_close()


with tempfile.TemporaryDirectory(prefix="substream 视频 $(literal) ") as directory:
    exercise(Path(directory), str(Path(sys.argv[1]).resolve()))
