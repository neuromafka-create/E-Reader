/**
 * Remove previous Vite/SvelteKit output so hashed chunks from older builds
 * cannot linger and confuse kit-id checks (or ship dead assets).
 */
import { existsSync, rmSync } from "node:fs";

for (const dir of ["build", ".svelte-kit/output"]) {
  if (existsSync(dir)) {
    rmSync(dir, { recursive: true, force: true });
    console.log(`[clean-frontend-build] removed ${dir}`);
  }
}
