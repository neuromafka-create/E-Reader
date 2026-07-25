// Static SPA for Tauri (no Node server at runtime).
// Prerender the shell so installed builds still show UI if JS is slow;
// Tauri IPC only runs in onMount / event handlers (never during build).
// See: https://v2.tauri.app/start/frontend/sveltekit/
export const ssr = true;
export const prerender = true;
export const csr = true;
