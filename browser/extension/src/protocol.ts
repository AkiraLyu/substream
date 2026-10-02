export const SAMPLE_RATE = 16_000;
export const FRAME_SAMPLES = 320;
export const HEADER_BYTES = 24;
export const MAX_BUFFERED_BYTES = 8 * (HEADER_BYTES + FRAME_SAMPLES * 2);

export interface Caption {
  segment_id: number;
  revision: number;
  start_ms: number;
  end_ms: number;
  stable_text: string;
  unstable_text: string;
  is_final: boolean;
}

export type ServerEvent =
  | {
      type: "ready";
      version: number;
      backend: { name: string; model: string; threads: number; languages: string[] };
    }
  | { type: "caption"; caption: Caption }
  | { type: "finished"; samples_processed: number; processing_ms: number }
  | { type: "error"; code: string; message: string };

export function encodeAudio(pcm: Int16Array, sequence: number, startSample: bigint): ArrayBuffer {
  if (pcm.length === 0 || pcm.length > 3200) throw new Error("Invalid PCM frame length");
  if (!Number.isInteger(sequence) || sequence < 0 || sequence > 0xffffffff)
    throw new Error("Invalid sequence");
  if (startSample < 0n || startSample + BigInt(pcm.length) > 0xffffffffffffffffn)
    throw new Error("Invalid sample clock");
  const buffer = new ArrayBuffer(HEADER_BYTES + pcm.length * 2);
  const view = new DataView(buffer);
  view.setUint32(0, 0x53425553, true); // SUBS
  view.setUint8(4, 1);
  view.setUint8(5, 1); // PCM16LE
  view.setUint8(6, 1); // mono
  view.setUint8(7, 0);
  view.setUint32(8, SAMPLE_RATE, true);
  view.setUint32(12, sequence, true);
  view.setBigUint64(16, startSample, true);
  pcm.forEach((sample, index) => view.setInt16(HEADER_BYTES + index * 2, sample, true));
  return buffer;
}
