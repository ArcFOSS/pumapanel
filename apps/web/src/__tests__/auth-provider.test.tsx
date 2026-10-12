import { act, renderHook, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { AuthProvider, useAuth } from "@/components/auth-provider";

import { jsonResponse, setupFetchStub, TEST_USER } from "./helpers";

const wrapper = ({ children }: { children: ReactNode }) => (
  <AuthProvider>{children}</AuthProvider>
);

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("AuthProvider", () => {
  it("loads the current user on mount", async () => {
    const fetchMock = setupFetchStub();
    fetchMock.mockResolvedValue(jsonResponse({ user: TEST_USER }));

    const { result } = renderHook(() => useAuth(), { wrapper });

    expect(result.current.status).toBe("loading");
    await waitFor(() => {
      expect(result.current.status).toBe("authenticated");
    });
    expect(result.current.user).toEqual(TEST_USER);
  });

  it("treats an unauthenticated session as anonymous", async () => {
    const fetchMock = setupFetchStub();
    fetchMock.mockResolvedValue(new Response(null, { status: 401 }));

    const { result } = renderHook(() => useAuth(), { wrapper });

    await waitFor(() => {
      expect(result.current.status).toBe("anonymous");
    });
    expect(result.current.user).toBeNull();
  });

  it("authenticates after a successful login", async () => {
    const fetchMock = setupFetchStub();
    fetchMock.mockResolvedValueOnce(new Response(null, { status: 401 }));
    fetchMock.mockResolvedValueOnce(jsonResponse({ user: TEST_USER }));

    const { result } = renderHook(() => useAuth(), { wrapper });
    await waitFor(() => {
      expect(result.current.status).toBe("anonymous");
    });

    await act(async () => {
      await result.current.login("user@example.com", "password123");
    });

    expect(result.current.status).toBe("authenticated");
    expect(result.current.user).toEqual(TEST_USER);
  });

  it("returns to anonymous on logout", async () => {
    const fetchMock = setupFetchStub();
    fetchMock.mockResolvedValueOnce(jsonResponse({ user: TEST_USER }));
    fetchMock.mockResolvedValueOnce(new Response(null, { status: 204 }));

    const { result } = renderHook(() => useAuth(), { wrapper });
    await waitFor(() => {
      expect(result.current.status).toBe("authenticated");
    });

    await act(async () => {
      await result.current.logout();
    });

    expect(result.current.status).toBe("anonymous");
    expect(result.current.user).toBeNull();
  });
});
