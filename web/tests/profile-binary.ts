import { resolve } from "node:path";

/**
 * Built executable in the active Cargo profile directory. scripts/test-ui.sh
 * exports RECOLLECT_PROFILE_DIR from `cargo metadata`, so CARGO_TARGET_DIR and
 * build.target-dir apply; a bare Playwright run keeps the in-tree default.
 */
export function profileBinary(name: string): string {
  return resolve(process.env.RECOLLECT_PROFILE_DIR ?? "../target/debug", name);
}
