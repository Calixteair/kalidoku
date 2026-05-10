import { api, ApiError } from "../api/client.js";
import type { components } from "../api/types.js";

type Me = components["schemas"]["Me"];

export const createAuthStore = () => {
  let me = $state<Me | null>(null);
  let loading = $state(false);
  let loaded = $state(false);

  const refresh = async (): Promise<void> => {
    loading = true;
    try {
      const data = await api.get("/api/me");
      me = data;
    } catch (err) {
      if (err instanceof ApiError && err.status === 401) {
        me = null;
      } else {
        // Backend may not be up — degrade gracefully without throwing.
        me = null;
      }
    } finally {
      loading = false;
      loaded = true;
    }
  };

  const logout = async (): Promise<void> => {
    try {
      await api.post("/api/auth/logout");
    } catch {
      /* swallow — clearing client state is enough */
    }
    me = null;
  };

  return {
    get me(): Me | null {
      return me;
    },
    get loading(): boolean {
      return loading;
    },
    get loaded(): boolean {
      return loaded;
    },
    get isAuthenticated(): boolean {
      return me !== null;
    },
    refresh,
    logout,
  };
};

export type AuthStore = ReturnType<typeof createAuthStore>;
