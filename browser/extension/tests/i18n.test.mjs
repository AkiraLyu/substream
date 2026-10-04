import assert from "node:assert/strict";
import { readFile, readdir } from "node:fs/promises";
import test from "node:test";

const publicFiles = new URL("../public/", import.meta.url);
const readJson = async (path) => JSON.parse(await readFile(new URL(path, publicFiles), "utf8"));

test("extension translations cover visible text and preserve message placeholders", async () => {
    const manifest = await readJson("manifest.json");
    const english = await readJson(`_locales/${manifest.default_locale}/messages.json`);
    const html = await readFile(new URL("popup.html", publicFiles), "utf8");
    const keys = [
        ...Array.from(html.matchAll(/data-i18n="([^"]+)"/g), (match) => match[1]),
        ...Array.from(JSON.stringify(manifest).matchAll(/__MSG_(\w+)__/g), (match) => match[1]),
    ];
    for (const key of keys) assert.ok(english[key]?.message, `missing fallback for ${key}`);

    for (const locale of await readdir(new URL("_locales/", publicFiles))) {
        const catalog = await readJson(`_locales/${locale}/messages.json`);
        for (const [key, entry] of Object.entries(catalog)) {
            assert.ok(english[key], `unknown message: ${locale}/${key}`);
            assert.ok(entry.message.trim(), `empty translation: ${locale}/${key}`);
            const parameters = (message) =>
                Array.from(message.matchAll(/\$([A-Z_]+)\$/gi), (m) => m[1].toLowerCase()).sort();
            assert.deepEqual(
                parameters(entry.message),
                parameters(english[key].message),
                `${locale}/${key}`,
            );
            const substitutions = (value) =>
                Object.fromEntries(
                    Object.entries(value ?? {}).map(([name, placeholder]) => [
                        name.toLowerCase(),
                        placeholder.content,
                    ]),
                );
            assert.deepEqual(
                substitutions(entry.placeholders),
                substitutions(english[key].placeholders),
                `${locale}/${key} substitutions`,
            );
        }
    }
});
