// Bundles the extension into the single file Spicetify loads.
// With SPICETIFY_EXTENSIONS_DIR set, also copies it there.

import { copyFile } from "node:fs/promises";
import { join } from "node:path";
import { build } from "esbuild";

const outfile = "dist/focuslock.js";

await build({
  entryPoints: ["src/main.ts"],
  bundle: true,
  format: "iife",
  target: "chrome120",
  outfile,
  banner: {
    js: "// Focus Lock extension. GPL-3.0-only. https://github.com/OWNER/focus-lock-for-spotify",
  },
});
console.log(`built ${outfile}`);

const dir = process.env.SPICETIFY_EXTENSIONS_DIR;
if (dir) {
  await copyFile(outfile, join(dir, "focuslock.js"));
  console.log(`copied to ${dir}`);
}
