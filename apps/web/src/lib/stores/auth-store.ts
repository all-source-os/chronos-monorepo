import { create } from "zustand";
import { persist } from "zustand/middleware";
import type { Tenant, User } from "@/lib/api/client";

interface AuthState {
  user: User | null;
  tenant: Tenant | null;
  coreApiKey: string | null;
  isLoading: boolean;
  isAuthenticated: boolean;
  error: string | null;

  // Actions
  setUser: (user: User | null) => void;
  setTenant: (tenant: Tenant | null) => void;
  setCoreApiKey: (key: string | null) => void;
  setLoading: (isLoading: boolean) => void;
  setError: (error: string | null) => void;
  login: (user: User, tenant: Tenant, coreApiKey?: string | null) => void;
  logout: () => void;
  reset: () => void;
}

const initialState = {
  user: null,
  tenant: null,
  coreApiKey: null,
  isLoading: true,
  isAuthenticated: false,
  error: null,
};

export const useAuthStore = create<AuthState>()(
  persist(
    (set) => ({
      ...initialState,

      setUser: (user) =>
        set({
          user,
          isAuthenticated: user !== null,
        }),

      setTenant: (tenant) => set({ tenant }),

      setCoreApiKey: (coreApiKey) => set({ coreApiKey }),

      setLoading: (isLoading) => set({ isLoading }),

      setError: (error) => set({ error }),

      login: (user, tenant, coreApiKey = null) =>
        set({
          user,
          tenant,
          coreApiKey,
          isAuthenticated: true,
          isLoading: false,
          error: null,
        }),

      logout: () =>
        set({
          ...initialState,
          isLoading: false,
        }),

      reset: () => set(initialState),
    }),
    {
      name: "auth-storage",
      partialize: (state) => ({
        user: state.user,
        tenant: state.tenant,
        coreApiKey: state.coreApiKey,
        isAuthenticated: state.isAuthenticated,
      }),
    }
  )
);
