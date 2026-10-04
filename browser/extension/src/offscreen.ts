import { message } from "./i18n.ts";
import { encodeAudio, MAX_BUFFERED_BYTES, SAMPLE_RATE } from "./protocol.ts";
import type { ServerEvent } from "./protocol.ts";

function report(event: object): void {
  void chrome.runtime.sendMessage({ target: "background", type: "event", event }).catch(() => {});
}

class Capture {
  private socket: WebSocket | null = null;
  private media: MediaStream | null = null;
  private captureContext: AudioContext | null = null;
  private monitorContext: AudioContext | null = null;
  private worklet: AudioWorkletNode | null = null;
  private sequence = 0;
  private sample = 0n;
  private disposed = false;
  private stopping = false;
  private flushed: (() => void) | null = null;
  private finishTimer: ReturnType<typeof setTimeout> | null = null;
  private ready: ServerEvent | null = null;

  async connect(token: string, source: string): Promise<void> {
    try {
      const socket = new WebSocket("ws://127.0.0.1:9743/v1/stream");
      this.socket = socket;
      await new Promise<void>((resolve, reject) => {
        const timer = setTimeout(() => reject(new Error(message("connectionTimeout"))), 30_000);
        socket.onopen = () =>
          socket.send(JSON.stringify({ type: "authenticate", version: 1, token, source }));
        socket.onerror = () => {
          clearTimeout(timer);
          reject(new Error(message("cannotConnect")));
        };
        socket.onclose = () => {
          clearTimeout(timer);
          reject(new Error(message("serviceDisconnected")));
          if (!this.disposed) this.fail(message("restartAfterDisconnect"));
        };
        socket.onmessage = ({ data }: MessageEvent<string>) => {
          try {
            const event = JSON.parse(data) as ServerEvent;
            if (event.type === "ready") {
              if (event.version !== 1) throw new Error(message("unsupportedProtocol"));
              clearTimeout(timer);
              this.ready = event;
              resolve();
            } else if (event.type === "error") {
              clearTimeout(timer);
              reject(new Error(event.message));
              this.fail(event.message);
            } else {
              report(event);
              if (event.type === "finished") this.dispose();
            }
          } catch (error) {
            clearTimeout(timer);
            reject(error);
            this.fail(message("invalidMessage"));
          }
        };
      });
      if (this.disposed) throw new Error(message("cancelled"));
    } catch (error) {
      this.dispose();
      throw error;
    }
  }

  async startAudio(streamId: string): Promise<void> {
    try {
      const socket = this.socket;
      if (this.disposed || !this.ready || socket?.readyState !== WebSocket.OPEN) {
        throw new Error(message("notReady"));
      }
      const media = await navigator.mediaDevices.getUserMedia({
        audio: {
          mandatory: { chromeMediaSource: "tab", chromeMediaSourceId: streamId },
        } as MediaTrackConstraints,
        video: false,
      });
      this.media = media;
      if (this.disposed) {
        media.getTracks().forEach((track) => track.stop());
        throw new Error(message("cancelled"));
      }
      this.monitorContext = new AudioContext();
      this.monitorContext.createMediaStreamSource(media).connect(this.monitorContext.destination);
      // Browser resampling includes anti-aliasing. Do not decimate by discarding samples.
      this.captureContext = new AudioContext({ sampleRate: SAMPLE_RATE });
      if (this.captureContext.sampleRate !== SAMPLE_RATE)
        throw new Error(message("sampleRateUnavailable"));
      await this.captureContext.audioWorklet.addModule("pcm-worklet.js");
      if (this.disposed) throw new Error(message("cancelled"));
      this.worklet = new AudioWorkletNode(this.captureContext, "substream-pcm");
      this.worklet.port.onmessage = ({ data }) => {
        if (data.type === "pcm") {
          if (socket.readyState !== WebSocket.OPEN) {
            this.fail(message("audioDisconnected"));
            return;
          }
          const pcm = new Int16Array(data.buffer as ArrayBuffer);
          const frame = encodeAudio(pcm, this.sequence, this.sample);
          if (socket.bufferedAmount + frame.byteLength > MAX_BUFFERED_BYTES) {
            this.fail(message("audioBacklog"));
            return;
          }
          socket.send(frame);
          this.sequence = (this.sequence + 1) >>> 0;
          this.sample += BigInt(pcm.length);
          this.worklet?.port.postMessage({ type: "credit" });
        } else if (data.type === "flushed") {
          this.flushed?.();
        } else if (data.type === "overloaded") {
          this.fail(message("queueFull"));
        }
      };
      this.worklet.onprocessorerror = () => this.fail(message("processorStopped"));
      const source = this.captureContext.createMediaStreamSource(media);
      source.connect(this.worklet);
      this.worklet.connect(this.captureContext.destination); // worklet output is silent
      for (const track of media.getTracks())
        track.onended = () => {
          void this.stop().catch((error: unknown) => this.fail(String(error)));
        };
      await Promise.all([this.captureContext.resume(), this.monitorContext.resume()]);
      if (this.disposed) throw new Error(message("cancelled"));
      report(this.ready);
    } catch (error) {
      this.dispose();
      throw error;
    }
  }

  cancel(): void {
    this.dispose();
  }

  async stop(): Promise<void> {
    if (this.disposed || this.stopping) return;
    this.stopping = true;
    report({ type: "stopping" });
    try {
      if (!this.worklet) throw new Error(message("audioNotStarted"));
      await new Promise<void>((resolve, reject) => {
        const timer = setTimeout(() => reject(new Error(message("flushTimeout"))), 1000);
        this.flushed = () => {
          clearTimeout(timer);
          resolve();
        };
        this.worklet?.port.postMessage({ type: "flush" });
      });
      if (this.disposed || this.socket?.readyState !== WebSocket.OPEN) return;
      this.socket.send(JSON.stringify({ type: "finish" }));
      this.releaseAudio();
      this.finishTimer = setTimeout(() => this.fail(message("finishTimeout")), 30_000);
    } catch (error) {
      this.fail(error instanceof Error ? error.message : String(error));
    }
  }

  private releaseAudio(): void {
    for (const track of this.media?.getTracks() ?? []) {
      track.onended = null;
      track.stop();
    }
    this.media = null;
    this.worklet?.disconnect();
    this.worklet?.port.close();
    this.worklet = null;
    if (this.captureContext) void this.captureContext.close();
    if (this.monitorContext) void this.monitorContext.close();
    this.captureContext = null;
    this.monitorContext = null;
  }

  private fail(message: string): void {
    if (this.disposed) return;
    report({ type: "error", message });
    this.dispose();
  }

  private dispose(): void {
    if (this.disposed) return;
    this.disposed = true;
    if (this.finishTimer) clearTimeout(this.finishTimer);
    this.releaseAudio();
    this.socket?.close();
    if (active === this) active = null;
  }
}

let active: Capture | null = null;
chrome.runtime.onMessage.addListener((request, sender, respond) => {
  if (sender.id !== chrome.runtime.id || request.target !== "offscreen") return false;
  const run = async () => {
    if (request.type === "stop") {
      await active?.stop();
      return;
    }
    if (request.type === "cancel") {
      active?.cancel();
      return;
    }
    if (request.type === "prepare") {
      if (active) throw new Error(message("alreadyActive"));
      active = new Capture();
      await active.connect(request.token as string, request.source as string);
      return;
    }
    if (request.type === "start") {
      if (!active) throw new Error(message("notPrepared"));
      await active.startAudio(request.streamId as string);
      return;
    }
    throw new Error(message("unknownCommand"));
  };
  void run()
    .then(() => respond({ ok: true }))
    .catch((error: unknown) =>
      respond({ ok: false, error: error instanceof Error ? error.message : String(error) }),
    );
  return true;
});
