import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import { encodeAudio } from "../src/protocol.ts";

test("Rust and TypeScript use the same golden binary frame", async () => {
    const golden = await readFile(new URL("../../../fixtures/audio-v1.bin", import.meta.url));
    assert.deepEqual(
        Buffer.from(encodeAudio(new Int16Array([-32768, 0, 32767]), 7, 2240n)),
        golden,
    );
});

test("sample offsets remain exact beyond the JavaScript number range", () => {
    const offset = 9007199254740993n;
    const frame = new DataView(encodeAudio(new Int16Array([1]), 0xffffffff, offset));
    assert.equal(frame.getBigUint64(16, true), offset);
    assert.equal(frame.getUint32(12, true), 0xffffffff);
    assert.throws(() => encodeAudio(new Int16Array(), 0, 0n));
    assert.throws(() => encodeAudio(new Int16Array(3201), 0, 0n));
    assert.throws(() => encodeAudio(new Int16Array(1), -1, 0n));
    assert.throws(() => encodeAudio(new Int16Array(1), 0, 0xffffffffffffffffn));
});
