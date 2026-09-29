import { mkdir, copyFile } from "node:fs/promises";
import { build } from "esbuild";

await mkdir("dist", { recursive: true });
await build({
    entryPoints: ["src/background.ts", "src/offscreen.ts", "src/popup.ts", "src/pcm-worklet.ts"],
    bundle: true,
    format: "esm",
    target: "chrome116",
    outdir: "dist",
});
for (const file of ["manifest.json", "popup.html", "popup.css", "offscreen.html"]) {
    await copyFile(`public/${file}`, `dist/${file}`);
}
