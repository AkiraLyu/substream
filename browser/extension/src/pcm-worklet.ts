import { FRAME_SAMPLES, SAMPLE_RATE } from "./protocol.ts";

class PcmProcessor extends AudioWorkletProcessor {
  private pcm = new Int16Array(FRAME_SAMPLES);
  private count = 0;
  private credits = 4;
  private active = true;

  constructor() {
    super();
    if (sampleRate !== SAMPLE_RATE) throw new Error("Substream requires a 16 kHz AudioContext");
    this.port.onmessage = ({ data }: MessageEvent<{ type: string }>) => {
      if (data.type === "credit") this.credits = Math.min(4, this.credits + 1);
      if (data.type === "flush") {
        if (this.count > 0) this.emit();
        this.active = false;
        this.port.postMessage({ type: "flushed" });
      }
    };
  }

  private emit(): void {
    if (this.credits === 0) {
      this.active = false;
      this.port.postMessage({ type: "overloaded" });
      return;
    }
    const pcm = this.count === FRAME_SAMPLES ? this.pcm : this.pcm.slice(0, this.count);
    this.port.postMessage({ type: "pcm", buffer: pcm.buffer }, [pcm.buffer]);
    this.credits -= 1;
    this.pcm = new Int16Array(FRAME_SAMPLES);
    this.count = 0;
  }

  process(inputs: Float32Array[][]): boolean {
    if (!this.active) return false;
    const channels = inputs[0];
    if (!channels?.[0]) return true;
    for (let i = 0; i < channels[0].length; i += 1) {
      let sample = 0;
      for (const channel of channels) sample += channel[i] ?? 0;
      sample /= channels.length;
      this.pcm[this.count++] = Math.max(-32768, Math.min(32767, Math.round(sample * 32768)));
      if (this.count === FRAME_SAMPLES) {
        this.emit();
        if (!this.active) break;
      }
    }
    return this.active;
  }
}

registerProcessor("substream-pcm", PcmProcessor);
