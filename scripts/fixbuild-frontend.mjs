/**
 * Make SvelteKit SPA shell safe for Tauri's custom protocol.
 *
 * Production builds emit absolute asset URLs (`/_app/...`). Under some
 * WebView entry URLs those resolve incorrectly and no JS loads → blank
 * white window. Rewrite to relative URLs after `vite build`.
 */
import { readFileSync, writeFileSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";

const buildDir = "build";
const indexPath = join(buildDir, "index.html");

let html = readFileSync(indexPath, "utf8");

const before = html;
html = html
  // Module / preload / stylesheet absolute app assets
  .replaceAll('"/_app/', '"./_app/')
  .replaceAll("'/_app/", "'./_app/")
  .replaceAll("(/_app/", "(./_app/")
  // Root static files
  .replaceAll('"/favicon.png"', '"./favicon.png"')
  .replaceAll("'/favicon.png'", "'./favicon.png'");

// Prefer a non-empty base when present so client router joins paths safely.
html = html.replace(
  /base:\s*""/g,
  'base: new URL("./", location.href).pathname.replace(/\\/index\\.html$/i, "/").replace(/\\/$/, "") || ""',
);

if (html === before) {
  console.warn("[postbuild-frontend] No absolute /_app paths found to rewrite.");
} else {
  writeFileSync(indexPath, html, "utf8");
  console.log("[postbuild-frontend] Rewrote absolute asset paths to relative in index.html");
}

// Quick sanity: entry modules must exist on disk.
const appDir = join(buildDir, "_app", "immutable", "entry");
for (const name of readdirSync(appDir)) {
  const full = join(appDir, name);
  if (statSync(full).isFile() && name.startsWith("start.")) {
    console.log(`[postbuild-frontend] entry OK: _app/immutable/entry/${name}`);
  }
}
