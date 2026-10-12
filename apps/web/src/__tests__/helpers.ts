import { vi } from "vitest";

import type { AuthUser, FieldError } from "@/lib/api";

/** JSON shape the API returns for auth endpoints and error responses. */
export interface JsonPayload {
  error?: {
    code?: string;
    fields?: FieldError[];
    message?: string;
  };
  user?: AuthUser;
}

export const jsonResponse = (body: JsonPayload, status = 200) =>
  Response.json(body, { status });

/**
 * Replaces the global `fetch` with a mock and returns it. Callers should
 * `vi.unstubAllGlobals()` in `afterEach`.
 */
export const setupFetchStub = () => {
  const fetchMock = vi.fn<typeof fetch>();
  vi.stubGlobal("fetch", fetchMock);
  return fetchMock;
};

export const TEST_USER: AuthUser = {
  createdAt: "2026-01-01T00:00:00.000Z",
  email: "user@example.com",
  id: "user-1",
  role: "user",
  username: "testuser",
};

export const ADMIN_USER: AuthUser = {
  ...TEST_USER,
  id: "admin-1",
  role: "admin",
  username: "testadmin",
};
