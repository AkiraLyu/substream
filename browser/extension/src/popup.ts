import { localizeDocument, message } from "./i18n.ts";
import type { Caption } from "./protocol.ts";

localizeDocument(document);

const token = document.querySelector<HTMLInputElement>("#token")!;
const start = document.querySelector<HTMLButtonElement>("#start")!;
const stop = document.querySelector<HTMLButtonElement>("#stop")!;
const status = document.querySelector<HTMLElement>("#status")!;
const stable = document.querySelector<HTMLElement>("#stable")!;
const partial = document.querySelector<HTMLElement>("#partial")!;

interface DisplayEvent {
  type: string;
  message?: string;
  caption?: Caption;
  backend?: { name: string; model: string; threads: number };
}

function show(value: unknown): void {
  if (!value || typeof value !== "object") return;
  const event = value as DisplayEvent;
  if (event.type === "caption" && event.caption) {
    stable.textContent = event.caption.stable_text;
    partial.textContent = event.caption.unstable_text;
    return;
  }
  start.disabled = ["starting", "ready", "stopping"].includes(event.type);
  stop.disabled = event.type !== "ready";
  if (event.type === "ready")
    status.textContent = message("recognizing", event.backend?.name ?? "");
  if (event.type === "starting") {
    status.textContent = message("starting");
    stable.textContent = "";
    partial.textContent = "";
  }
  if (event.type === "stopping") status.textContent = message("stopping");
  if (event.type === "finished") status.textContent = message("finished");
  if (event.type === "error") status.textContent = event.message ?? message("connectionFailed");
}

start.addEventListener("click", () => {
  show({ type: "starting" });
  void (async () => {
    const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
    if (tab?.id === undefined) throw new Error(message("noTab"));
    const reply = await chrome.runtime.sendMessage({
      target: "background",
      type: "start",
      tabId: tab.id,
      token: token.value.trim(),
    });
    if (!reply?.ok) throw new Error(reply?.error ?? message("startFailed"));
  })().catch((error: unknown) => show({ type: "error", message: String(error) }));
});

stop.addEventListener("click", () => {
  show({ type: "stopping" });
  void chrome.runtime
    .sendMessage({ target: "background", type: "stop" })
    .then((reply) => {
      if (!reply?.ok) throw new Error(reply?.error ?? message("stopFailed"));
    })
    .catch((error: unknown) => show({ type: "error", message: String(error) }));
});

chrome.storage.onChanged.addListener((changes, area) => {
  if (area !== "session") return;
  if (changes.statusEvent) show(changes.statusEvent.newValue);
  if (changes.lastCaption) show(changes.lastCaption.newValue);
});
void chrome.storage.session.get(["token", "statusEvent", "lastCaption"]).then((saved) => {
  token.value = (saved.token as string | undefined) ?? "";
  show(saved.statusEvent);
  show(saved.lastCaption);
});
