import type english from "../public/_locales/en/messages.json";

export type MessageKey = keyof typeof english;

/** Chrome selects the UI locale and falls back to the manifest's default locale. */
export function message(key: MessageKey, substitutions?: string | string[]): string {
  return chrome.i18n.getMessage(key, substitutions);
}

export function localizeDocument(document: Document): void {
  document.documentElement.lang = message("language");
  document.querySelectorAll<HTMLElement>("[data-i18n]").forEach((element) => {
    element.textContent = message(element.dataset.i18n as MessageKey);
  });
}
