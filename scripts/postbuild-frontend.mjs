/**
 * Post-process SvelteKit static output for Tauri WebView.
 *
 * Keep SvelteKit's boot script intact (kit id + hydrate payload).
 * Only:
 *  - absolute `/_app/...` asset URLs
 *  - empty `base: ""` when relative base helper is emitted
 *  - inline CSS into <head>
 *  - verify kit id using the start entry actually imported by index.html
 *    (not the first leftover chunk from a previous build)
 */
import {
  readFileSync,
  writeFileSync,
  readdirSync,
  existsSync,
} from "node:fs";
import { join, dirname } from "node:path";
import { spawnSync } from "node:child_process";

const buildDir = "build";
const indexPath = join(buildDir, "index.html");
const assetsDir = join(buildDir, "_app", "immutable", "assets");

function read(p) {
  return readFileSync(p, "utf8");
}

/** Resolve /_app/... or ./_app/... path from index.html to a filesystem path. */
function appUrlToFs(urlPath) {
  const clean = urlPath.replace(/^\.\//, "/").replace(/^\//, "");
  return join(buildDir, clean);
}

/** Follow start.xxx.js → chunk that references globalThis.__sveltekit_*. */
function kitIdFromStartEntry(html) {
  const importMatch = html.match(
    /import\((["'`])(\/?\.?\/?_app\/immutable\/entry\/start\.[^"'`]+)\1\)/,
  );
  if (!importMatch) return { htmlId: null, chunkId: null, startPath: null };

  const startUrl = importMatch[2].replace(/^\.\//, "/");
  const startFs = appUrlToFs(startUrl);
  if (!existsSync(startFs)) {
    return { htmlId: null, chunkId: null, startPath: startUrl };
  }

  const startBody = read(startFs);
  // e.g. import{...}from"../chunks/XXXX.js"
  const chunkRel = startBody.match(/from\s*["'](\.\.\/chunks\/[^"']+)["']/);
  let chunkId = null;
  if (chunkRel) {
    const chunkFs = join(dirname(startFs), chunkRel[1]);
    if (existsSync(chunkFs)) {
      const m = read(chunkFs).match(/globalThis\.__sveltekit_([\w$]+)/);
      if (m) chunkId = m[1];
    }
  }
  // Fallback: any __sveltekit_ in the start graph file itself
  if (!chunkId) {
    const m = startBody.match(/__sveltekit_([\w$]+)/);
    if (m) chunkId = m[1];
  }

  const htmlId = (html.match(/__sveltekit_([\w$]+)/) || [])[1] || null;
  return { htmlId, chunkId, startPath: startUrl };
}

let html = read(indexPath);

// Absolute asset URLs for https://tauri.localhost
html = html
  .replaceAll('"./_app/', '"/_app/')
  .replaceAll("'./_app/", "'/_app/")
  .replaceAll("(./_app/", "(/_app/")
  .replaceAll("./_app/", "/_app/")
  .replaceAll('href="./favicon.png"', 'href="/favicon.png"')
  .replaceAll("href='./favicon.png'", "href='/favicon.png'");

// Stock relative base helper → empty base
html = html.replace(
  /base:\s*new URL\("\.",\s*location\)\.pathname\.slice\(0,\s*-1\)/g,
  'base: ""',
);
html = html.replace(
  /base:\s*new URL\("\.",\s*location\)\.pathname\.slice\(0,-1\)/g,
  'base: ""',
);

// Bind kit global on globalThis (keep the same identifier)
html = html.replace(
  /(\n\s*)(__sveltekit_[\w$]+)(\s*=\s*\{)/g,
  "$1globalThis.$2$3",
);

// Inline CSS
const cssFiles = existsSync(assetsDir)
  ? readdirSync(assetsDir).filter((f) => f.endsWith(".css")).sort()
  : [];

if (cssFiles.length) {
  const cssBody = cssFiles
    .map((f) => read(join(assetsDir, f)))
    .join("\n");
  html = html.replace(
    /\s*<link href="\/_app\/immutable\/assets\/[^"]+\.css" rel="stylesheet">/g,
    "",
  );
  html = html.replace(
    /\s*<style data-ereader-critical="1">[\s\S]*?<\/style>/g,
    "",
  );
  html = html.replace(
    "</head>",
    `\t\t<style data-ereader-critical="1">\n${cssBody}\n</style>\n  </head>`,
  );
  console.log(
    `[postbuild-frontend] Inlined ${cssFiles.length} CSS file(s) (${cssBody.length} chars)`,
  );
}

// If HTML kit id drifted from the start-entry chunk (stale build artifacts),
// rewrite HTML to the chunk id that will actually run.
const { htmlId, chunkId, startPath } = kitIdFromStartEntry(html);
if (htmlId && chunkId && htmlId !== chunkId) {
  console.warn(
    `[postbuild-frontend] kit id mismatch html=${htmlId} start-chunk=${chunkId} — aligning HTML to chunk`,
  );
  html = html.split(`__sveltekit_${htmlId}`).join(`__sveltekit_${chunkId}`);
} else if (htmlId && chunkId) {
  console.log(
    `[postbuild-frontend] kit id OK: __sveltekit_${htmlId} (via ${startPath})`,
  );
} else {
  console.warn(
    `[postbuild-frontend] Could not fully verify kit id (html=${htmlId}, chunk=${chunkId}, start=${startPath})`,
  );
}

writeFileSync(indexPath, html, "utf8");

// Syntax-check boot script
const bootMatch = html.match(
  /<script>\s*(\{[\s\S]*__sveltekit_[\s\S]*\})\s*<\/script>/,
);
if (bootMatch) {
  const tmp = join(buildDir, "_boot-check.js");
  writeFileSync(tmp, bootMatch[1], "utf8");
  const check = spawnSync(process.execPath, ["--check", tmp], {
    encoding: "utf8",
  });
  if (check.status !== 0) {
    console.error("[postbuild-frontend] Boot script syntax error:");
    console.error(check.stderr || check.stdout);
    process.exit(1);
  }
  console.log("[postbuild-frontend] Boot script syntax OK");
}

// Prune obviously stale hashed chunks? Too risky. Clean happens in prebuild.

console.log("[postbuild-frontend] Done (stock SvelteKit hydrate boot preserved)");
