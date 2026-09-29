import assert from "node:assert/strict";
import vm from "node:vm";
import test from "node:test";
import { build } from "esbuild";

const { outputFiles } = await build({
    entryPoints: ["src/background.ts"],
    bundle: true,
    write: false,
    format: "iife",
});

function background({ failCapture = false } = {}) {
    const calls = [];
    const saved = {};
    let listener;
    let finishPreparation;
    const preparation = new Promise((resolve) => {
        finishPreparation = resolve;
    });
    const chrome = {
        runtime: {
            id: "test",
            ContextType: { OFFSCREEN_DOCUMENT: "offscreen" },
            onMessage: {
                addListener(callback) {
                    listener = callback;
                },
            },
            async getContexts() {
                return [];
            },
            async sendMessage(message) {
                calls.push(message.type);
                if (message.type === "prepare") await preparation;
                return { ok: true };
            },
        },
        storage: {
            session: {
                async get() {
                    return saved;
                },
                async set(values) {
                    Object.assign(saved, values);
                },
            },
        },
        offscreen: {
            Reason: { USER_MEDIA: "user-media" },
            async createDocument() {
                calls.push("document");
            },
        },
        tabCapture: {
            async getMediaStreamId() {
                calls.push("stream-id");
                if (failCapture) throw new Error("capture permission denied");
                return "short-lived-id";
            },
        },
    };
    vm.runInNewContext(outputFiles[0].text, { chrome });
    return {
        calls,
        saved,
        finishPreparation,
        request(message) {
            return new Promise((resolve) =>
                listener({ target: "background", ...message }, { id: "test" }, resolve),
            );
        },
    };
}

test("model readiness precedes short-lived tab capture ID allocation", async () => {
    const runtime = background();
    const response = runtime.request({ type: "start", token: "a".repeat(64), tabId: 1 });
    await new Promise((resolve) => setImmediate(resolve));
    assert.deepEqual(runtime.calls, ["document", "prepare"]);
    runtime.finishPreparation();
    assert.equal((await response).ok, true);
    assert.deepEqual(runtime.calls, ["document", "prepare", "stream-id", "start"]);
});

test("capture permission failures release the already prepared inference session", async () => {
    const runtime = background({ failCapture: true });
    runtime.finishPreparation();
    const response = await runtime.request({ type: "start", token: "a".repeat(64), tabId: 1 });
    assert.equal(response.ok, false);
    assert.equal(runtime.calls.at(-1), "cancel");
    assert.equal(runtime.saved.statusEvent.type, "error");
});

test("caption updates preserve the backend status needed when reopening the popup", async () => {
    const runtime = background();
    await runtime.request({
        type: "event",
        event: { type: "ready", backend: { synthetic: true } },
    });
    await runtime.request({
        type: "event",
        event: { type: "caption", caption: { stable_text: "demo" } },
    });
    assert.equal(runtime.saved.statusEvent.type, "ready");
    assert.equal(runtime.saved.lastCaption.caption.stable_text, "demo");
    const response = await runtime.request({ type: "start", token: "a".repeat(64), tabId: 1 });
    assert.equal(response.ok, false);
    assert.equal(runtime.saved.statusEvent.type, "ready");
});
