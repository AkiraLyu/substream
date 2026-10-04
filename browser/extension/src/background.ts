import { validateToken } from "./auth.ts";
import { message } from "./i18n.ts";
import { handleVideoAction } from "./video.ts";
import type { VideoAction } from "./video.ts";

type Request =
  | { target: "background"; type: "video"; token: string; action: VideoAction }
  | { target: "background"; type: "start"; token: string; tabId: number }
  | { target: "background"; type: "stop" }
  | { target: "background"; type: "event"; event: Record<string, unknown> };

let starting = false;

async function handle(request: Exclude<Request, { type: "video" }>): Promise<void> {
  if (request.type === "event") {
    const key = request.event.type === "caption" ? "lastCaption" : "statusEvent";
    await chrome.storage.session.set({ [key]: request.event });
    return;
  }
  if (request.type === "stop") {
    const reply = await chrome.runtime.sendMessage({ target: "offscreen", type: "stop" });
    if (!reply?.ok) throw new Error(reply?.error ?? message("noCapture"));
    return;
  }
  if (starting) throw new Error(message("alreadyStarting"));
  validateToken(request.token);
  starting = true;
  let attempted = false;
  let prepared = false;
  try {
    const { statusEvent } = await chrome.storage.session.get("statusEvent");
    const status = statusEvent as { type?: string } | undefined;
    if (["ready", "starting", "stopping"].includes(status?.type ?? ""))
      throw new Error(message("alreadyActive"));
    attempted = true;
    await chrome.storage.session.set({
      token: request.token,
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
    const tab = await chrome.tabs.get(request.tabId);
    // Stream IDs expire within seconds. Load the model before allocating an ID.
    const preparation = await chrome.runtime.sendMessage({
      target: "offscreen",
      type: "prepare",
      token: request.token,
      source: Array.from(tab.title ?? message("tabTitle", String(request.tabId)))
        .slice(0, 120)
        .join(""),
    });
    if (!preparation?.ok) throw new Error(preparation?.error ?? message("preparationFailed"));
    prepared = true;
    const streamId = await chrome.tabCapture.getMediaStreamId({ targetTabId: request.tabId });
    const reply = await chrome.runtime.sendMessage({
      target: "offscreen",
      type: "start",
      streamId,
    });
    if (!reply?.ok) throw new Error(reply?.error ?? message("startFailed"));
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

chrome.runtime.onMessage.addListener((request: Request, sender, respond) => {
  if (sender.id !== chrome.runtime.id || request.target !== "background") return false;
  const operation =
    request.type === "video" ? handleVideoAction(request.token, request.action) : handle(request);
  void operation
    .then((data) => respond({ ok: true, data }))
    .catch((error: unknown) => {
      const text = error instanceof Error ? error.message : String(error);
      respond({ ok: false, error: text });
    });
  return true;
});
