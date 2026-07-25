// Tauri doesn't have a Node.js server to do proper SSR
// so we use adapter-static with a fallback to index.html to put the site in SPA mode
// See: https://svelte.dev/docs/kit/single-page-apps
// See: https://v2.tauri.app/start/frontend/sveltekit/ for more info
import adapter from "@sveltejs/adapter-static";
import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";

/** @type {import('@sveltejs/kit').Config} */
const config = {
  preprocess: vitePreprocess(),
  kit: {
    adapter: adapter({
      // Do NOT use fallback: "index.html" — it overwrites the prerendered
      // shell and leaves a nearly empty SPA page (blank window risk in Tauri).
      // This app is a single route (`/`); prerender covers it.
      strict: true,
    }),
    // Absolute `/_app/...` paths (not relative). Relative URLs break module
    // + CSS loading under Tauri's https://tauri.localhost origin.
    paths: {
      relative: false,
    },
  },
};

export default config;
