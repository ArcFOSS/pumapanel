import { afterEach, describe, expect, it, vi } from "vitest";

import { ApiError, authApi } from "@/lib/api";

import { jsonResponse, setupFetchStub, TEST_USER } from "./helpers";

afterEach(() => {
  vi.unstubAllGlobals();
});

const validationErrorResponse = () =>
  jsonResponse(
    {
      error: {
        code: "validation_error",
        fields: [{ field: "email", message: "Enter a valid email address" }],
        message: "Invalid input",
      },
    },
    400
  );

describe("authApi", () => {
  it("posts credentials to the login endpoint and returns the user", async () => {
    const fetchMock = setupFetchStub();
    fetchMock.mockResolvedValue(jsonResponse({ user: TEST_USER }));

    const response = await authApi.login("user@example.com", "password123");

    expect(response.user).toEqual(TEST_USER);
    const [[url, init]] = fetchMock.mock.calls;
    expect(url).toBe("/api/auth/login");
    expect(init?.method).toBe("POST");
    expect(init?.credentials).toBe("same-origin");
    expect(init?.body).toBe(
      JSON.stringify({ email: "user@example.com", password: "password123" })
    );
  });

  it("throws an ApiError carrying field details on validation failures", async () => {
    const fetchMock = setupFetchStub();
    fetchMock.mockResolvedValueOnce(validationErrorResponse());
    fetchMock.mockResolvedValueOnce(validationErrorResponse());

    await expect(authApi.register("bad", "short")).rejects.toBeInstanceOf(
      ApiError
    );
    await expect(authApi.register("bad", "short")).rejects.toMatchObject({
      code: "validation_error",
      fields: [{ field: "email", message: "Enter a valid email address" }],
      status: 400,
    });
  });

  it("reports a network error when the request cannot be sent", async () => {
    const fetchMock = setupFetchStub();
    fetchMock.mockRejectedValue(new TypeError("Failed to fetch"));

    await expect(authApi.me()).rejects.toMatchObject({
      code: "network_error",
      status: 0,
    });
  });

  it("falls back to a generic error for a non-JSON error body", async () => {
    const fetchMock = setupFetchStub();
    fetchMock.mockResolvedValue(
      new Response("upstream unavailable", { status: 502 })
    );

    await expect(authApi.me()).rejects.toMatchObject({
      code: "unknown_error",
      status: 502,
    });
  });

  it("resolves without a body for logout", async () => {
    const fetchMock = setupFetchStub();
    fetchMock.mockResolvedValue(new Response(null, { status: 204 }));

    await expect(authApi.logout()).resolves.toBeNull();
  });
});
