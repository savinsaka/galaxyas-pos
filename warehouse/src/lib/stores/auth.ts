import { writable, get } from "svelte/store";
import type { ModuleKey, User } from "$lib/types";

export const currentUser = writable<User | null>(null);

export function setUser(u: User | null) {
  currentUser.set(u);
}

export function logout() {
  currentUser.set(null);
}

/** Cek apakah user aktif punya akses ke modul tertentu. Admin selalu boleh. */
export function can(perm: ModuleKey): boolean {
  const u = get(currentUser);
  if (!u) return false;
  if (u.role === "admin") return true;
  return u.permissions.includes(perm);
}
