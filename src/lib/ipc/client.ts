import { mockInvoke } from "./mock";

/**
 * Detect whether we are running inside the Tauri webview. When developing in a
 * plain browser (`npm run dev` without `tauri dev`) we fall back to an in-memory
 * mock so the UI is fully demoable without the Rust backend.
 */
export function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

/**
 * Thin typed wrapper around Tauri's `invoke`. All command names and argument
 * shapes are centralized in `commands.ts`; this function is the single place
 * that talks to the Rust side (or the mock in browser dev).
 */
export async function ipcInvoke<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  if (isTauri()) {
    // Imported lazily so the browser dev build never pulls Tauri internals.
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<T>(command, args);
  }
  return mockInvoke<T>(command, args);
}
