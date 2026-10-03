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

    class MediaHandler(http.server.BaseHTTPRequestHandler):
        def do_GET(self):
            if self.headers.get("Cookie") != "session=fixture-login":
                self.send_error(403)
                return
            self.send_response(200)
            self.send_header("Content-Type", "video/x-matroska")
            self.send_header("Content-Length", str(len(content)))
            self.end_headers()
            self.wfile.write(content)

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
    token_file = root / "token"
    run(binary, "token", "--output", str(token_file))
    token = token_file.read_text().strip()
    origin = "chrome-extension://test"
    with socket.socket() as reservation:
        reservation.bind(("127.0.0.1", 0))
        address = f"127.0.0.1:{reservation.getsockname()[1]}"
    service = subprocess.Popen(
        [binary, "serve", "--listen", address, "--video-config", str(config),
         "--token-file", str(token_file), "--allow-origin", origin],
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

        # CLI uses the same result contract and preserves previous output on a repeated request.
        output = run(binary, "video", url, "--config", str(config))
        cli_job = json.loads(output.stdout.splitlines()[-1])
        assert cli_job["stage"] == "completed", cli_job
        assert cli_job["result"]["directory"] != job["result"]["directory"]
        assert check_result(cli_job["result"], url) == document
        assert check_result(job["result"], url) == document
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
