import assert from "node:assert/strict";
import vm from "node:vm";
import test from "node:test";
import { build } from "esbuild";

const { outputFiles } = await build({
    entryPoints: ["src/pcm-worklet.ts"],
    bundle: true,
    write: false,
    format: "iife",
});

function processor() {
    const messages = [];
    let Processor;
    const context = {
        sampleRate: 16000,
        AudioWorkletProcessor: class {
            port = {
                onmessage: null,
                postMessage(message, transfer = []) {
                    messages.push(structuredClone(message, { transfer }));
                },
            };
        },
        registerProcessor(_name, implementation) {
            Processor = implementation;
        },
    };
    vm.runInNewContext(outputFiles[0].text, context);
    return { instance: new Processor(), messages };
}

test("stereo audio is downmixed without losing the tail on stop", () => {
    const { instance, messages } = processor();
    const blocks = 137;
    let acknowledged = 0;
    for (let i = 0; i < blocks; i += 1) {
        instance.process([[new Float32Array(128).fill(0.5), new Float32Array(128).fill(-0.25)]]);
        for (const message of messages.slice(acknowledged)) {
            if (message.type === "pcm") instance.port.onmessage({ data: { type: "credit" } });
        }
        acknowledged = messages.length;
    }
    instance.port.onmessage({ data: { type: "flush" } });
    const samples = messages
        .filter((message) => message.type === "pcm")
        .flatMap((message) => Array.from(new Int16Array(message.buffer)));
    assert.equal(samples.length, blocks * 128);
    assert.ok(samples.every((sample) => Math.abs(sample / 32768 - 0.125) <= 1 / 32768));
    assert.ok(messages.some((message) => message.type === "flushed"));
});

test("stalled audio delivery reports an error and stops buffering", () => {
    const { instance, messages } = processor();
    const feedAudio = () => {
        for (let i = 0; i < 200; i += 1) instance.process([[new Float32Array(128)]]);
    };
    feedAudio();
    assert.ok(messages.some((message) => message.type === "overloaded"));
    const buffered = messages.filter((message) => message.type === "pcm").length;
    feedAudio();
    assert.equal(messages.filter((message) => message.type === "pcm").length, buffered);
});
