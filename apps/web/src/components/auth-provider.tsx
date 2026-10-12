"use client";

import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
} from "react";
import type { ReactNode } from "react";

import { authApi } from "@/lib/api";
import type { AuthUser } from "@/lib/api";

type AuthStatus = "loading" | "authenticated" | "anonymous";

interface AuthState {
  status: AuthStatus;
  user: AuthUser | null;
}

interface AuthContextValue extends AuthState {
  login: (email: string, password: string) => Promise<void>;
  register: (email: string, password: string) => Promise<void>;
  logout: () => Promise<void>;
}

const AuthContext = createContext<AuthContextValue | null>(null);

const ANONYMOUS: AuthState = { status: "anonymous", user: null };

/**
 * Holds the client's view of the authenticated session.
 *
 * The session token lives in an HttpOnly cookie, so this provider cannot read
 * it directly; it asks `GET /api/auth/me` on mount instead. That means a valid
 * session survives page reloads and refreshes. A `401` means "not signed in";
 * any other failure is also treated as unauthenticated rather than crashing.
 */
export const AuthProvider = ({ children }: { children: ReactNode }) => {
  const [state, setState] = useState<AuthState>({
    status: "loading",
    user: null,
  });

  useEffect(() => {
    let active = true;

    const load = async () => {
      try {
        const { user } = await authApi.me();
        if (active) {
          setState({ status: "authenticated", user });
        }
      } catch {
        if (active) {
          setState(ANONYMOUS);
        }
      }
    };

    void load();

    return () => {
      active = false;
    };
  }, []);

  const login = useCallback(async (email: string, password: string) => {
    const { user } = await authApi.login(email, password);
    setState({ status: "authenticated", user });
  }, []);

  const register = useCallback(async (email: string, password: string) => {
    const { user } = await authApi.register(email, password);
    setState({ status: "authenticated", user });
  }, []);

  const logout = useCallback(async () => {
    try {
      await authApi.logout();
    } catch {
      // Clearing the client state is what matters; a failed request (e.g. the
      // server is unreachable) should not block signing out.
    }
    setState(ANONYMOUS);
  }, []);

  const value = useMemo<AuthContextValue>(
    () => ({ ...state, login, logout, register }),
    [state, login, logout, register]
  );

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
};

export const useAuth = (): AuthContextValue => {
  const context = useContext(AuthContext);
  if (!context) {
    throw new Error("useAuth must be used within an AuthProvider");
  }
  return context;
};
