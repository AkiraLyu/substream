export {};

type Request =
  | { target: "background"; type: "start"; token: string; tabId: number }
  | { target: "background"; type: "stop" }
  | { target: "background"; type: "event"; event: Record<string, unknown> };

let starting = false;

async function handle(message: Request): Promise<void> {
  if (message.type === "event") {
    const key = message.event.type === "caption" ? "lastCaption" : "statusEvent";
    await chrome.storage.session.set({ [key]: message.event });
    return;
  }
  if (message.type === "stop") {
    const reply = await chrome.runtime.sendMessage({ target: "offscreen", type: "stop" });
    if (!reply?.ok) throw new Error(reply?.error ?? "No capture is active");
    return;
  }
  if (starting) throw new Error("Capture is already starting");
  if (!/^[a-fA-F0-9]{64}$/.test(message.token)) throw new Error("配对令牌需要 64 位十六进制字符");
  starting = true;
  let attempted = false;
  let prepared = false;
  try {
    const { statusEvent } = await chrome.storage.session.get("statusEvent");
    const status = statusEvent as { type?: string } | undefined;
    if (["ready", "starting", "stopping"].includes(status?.type ?? ""))
      throw new Error("Capture is already active");
    attempted = true;
    await chrome.storage.session.set({
      token: message.token,
      statusEvent: { type: "starting" },
      lastCaption: null,
    });
    const contexts = await chrome.runtime.getContexts({
      contextTypes: [chrome.runtime.ContextType.OFFSCREEN_DOCUMENT],
    });
    if (contexts.length === 0) {
      await chrome.offscreen.createDocument({
        url: "offscreen.html",
        reasons: [chrome.offscreen.Reason.USER_MEDIA],
        justification: "Capture user-selected tab audio for local live captions",
      });
    }
    // Stream IDs expire within seconds. Load the model before allocating an ID.
    const preparation = await chrome.runtime.sendMessage({
      target: "offscreen",
      type: "prepare",
      token: message.token,
    });
    if (!preparation?.ok) throw new Error(preparation?.error ?? "Daemon preparation failed");
    prepared = true;
    const streamId = await chrome.tabCapture.getMediaStreamId({ targetTabId: message.tabId });
    const reply = await chrome.runtime.sendMessage({
      target: "offscreen",
      type: "start",
      streamId,
    });
    if (!reply?.ok) throw new Error(reply?.error ?? "Capture failed to start");
  } catch (error) {
    if (prepared)
      await chrome.runtime.sendMessage({ target: "offscreen", type: "cancel" }).catch(() => {});
    if (attempted)
      await chrome.storage.session.set({
        statusEvent: {
          type: "error",
          message: error instanceof Error ? error.message : String(error),
        },
      });
    throw error;
  } finally {
    starting = false;
  }
}

chrome.runtime.onMessage.addListener((message: Request, sender, respond) => {
  if (sender.id !== chrome.runtime.id || message.target !== "background") return false;
  void handle(message)
    .then(() => respond({ ok: true }))
    .catch((error: unknown) => {
      const text = error instanceof Error ? error.message : String(error);
      respond({ ok: false, error: text });
    });
  return true;
});
