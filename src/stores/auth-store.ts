import { create } from "zustand";
import type { Session, User, UserRole } from "@/types";
import { ipc } from "@/lib/ipc/commands";

interface AuthState {
  session: Session | null;
  loading: boolean;
  error: string | null;
  init: () => Promise<void>;
  login: (username: string, password: string) => Promise<void>;
  logout: () => Promise<void>;
  hasRole: (...roles: UserRole[]) => boolean;
  user: () => User | null;
}

export const useAuthStore = create<AuthState>((set, get) => ({
  session: null,
  loading: true,
  error: null,

  init: async () => {
    set({ loading: true });
    try {
      const session = await ipc.currentSession();
      set({ session, loading: false, error: null });
    } catch (e) {
      set({ loading: false, error: String(e) });
    }
  },

  login: async (username, password) => {
    set({ loading: true, error: null });
    try {
      const session = await ipc.login(username, password);
      set({ session, loading: false });
    } catch (e) {
      set({ loading: false, error: String(e) });
      throw e;
    }
  },

  logout: async () => {
    await ipc.logout();
    set({ session: null });
  },

  hasRole: (...roles) => {
    const role = get().session?.user.role;
    return role ? roles.includes(role) : false;
  },

  user: () => get().session?.user ?? null,
}));
