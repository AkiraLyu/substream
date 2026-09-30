import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import { encodeAudio } from "../src/protocol.ts";

test("browser audio matches the shared protocol sample", async () => {
    const golden = await readFile(new URL("../../../fixtures/audio-v1.bin", import.meta.url));
    assert.deepEqual(
        Buffer.from(encodeAudio(new Int16Array([-32768, 0, 32767]), 7, 2240n)),
        golden,
    );
});
