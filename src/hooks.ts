/**
 * Shared hooks (client + server build graph).
 *
 * Tauri production often opens the webview at `/index.html`.
 * Our SPA only registers `/`, so without this rewrite SvelteKit
 * never mounts the page → blank white window. `tauri dev` uses
 * `http://localhost:1420/` and is unaffected.
 */
export function reroute({ url }: { url: URL }) {
  const path = url.pathname;
  if (path === "/index.html" || path.endsWith("/index.html") || path === "") {
    return "/";
  }
}
