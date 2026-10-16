// Bundles the extension into the single file Spicetify loads.
// With SPICETIFY_EXTENSIONS_DIR set, also copies it there.

import { copyFile } from "node:fs/promises";
import { join } from "node:path";
import { build } from "esbuild";

const outfile = "dist/tunelock.js";

await build({
  entryPoints: ["src/main.ts"],
  bundle: true,
  format: "iife",
  target: "chrome120",
  outfile,
  banner: {
    js: "// Tunelock extension. GPL-3.0-only. https://github.com/OWNER/tunelock",
  },
});
console.log(`built ${outfile}`);

const dir = process.env.SPICETIFY_EXTENSIONS_DIR;
if (dir) {
  await copyFile(outfile, join(dir, "tunelock.js"));
  console.log(`copied to ${dir}`);
}
