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

function processor(rate = 16000) {
    const messages = [];
    let Processor;
    const context = {
        sampleRate: rate,
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

test("20 ms frames downmix stereo and flush the final partial frame", () => {
    const { instance, messages } = processor();
    for (let i = 0; i < 3; i += 1) {
        instance.process([[new Float32Array(128).fill(0.5), new Float32Array(128).fill(-0.25)]]);
    }
    assert.equal(messages.length, 1);
    assert.equal(new Int16Array(messages[0].buffer).length, 320);
    assert.ok(new Int16Array(messages[0].buffer).every((n) => n === 4096));
    instance.port.onmessage({ data: { type: "flush" } });
    assert.equal(new Int16Array(messages[1].buffer).length, 64);
    assert.equal(messages[2].type, "flushed");
    assert.equal(instance.process([]), false);
});

test("a stalled consumer cannot grow the worklet message queue indefinitely", () => {
    const { instance, messages } = processor();
    for (let i = 0; i < 5; i += 1) instance.process([[new Float32Array(320)]]);
    assert.equal(messages.filter((m) => m.type === "pcm").length, 4);
    assert.equal(messages.filter((m) => m.type === "overloaded").length, 1);
    assert.equal(instance.process([]), false);
});

test("credits release capacity and the sample rate contract is enforced", () => {
    const { instance, messages } = processor();
    for (let i = 0; i < 20; i += 1) {
        instance.process([[new Float32Array(320)]]);
        instance.port.onmessage({ data: { type: "credit" } });
    }
    assert.equal(messages.length, 20);
    assert.throws(() => processor(48000), /16 kHz/);
});
