import { describe, expect, it } from "vitest";

import { guardRoute, SESSION_COOKIE } from "@/lib/route-guard";

describe("guardRoute", () => {
  it("redirects unauthenticated visitors away from the dashboard", () => {
    expect(guardRoute("/dashboard", false)).toEqual({ redirectTo: "/login" });
    expect(guardRoute("/dashboard/overview", false)).toEqual({
      redirectTo: "/login",
    });
  });

  it("lets visitors with a session through protected paths", () => {
    expect(guardRoute("/dashboard", true)).toBeNull();
    expect(guardRoute("/dashboard/overview", true)).toBeNull();
  });

  it("treats admin pages as protected paths", () => {
    expect(guardRoute("/admin", false)).toEqual({ redirectTo: "/login" });
    expect(guardRoute("/admin/servers", false)).toEqual({
      redirectTo: "/login",
    });
    expect(guardRoute("/admin/u/user-1", false)).toEqual({
      redirectTo: "/login",
    });
    expect(guardRoute("/admin/edit/u/user-1", false)).toEqual({
      redirectTo: "/login",
    });
    expect(guardRoute("/admin/servers", true)).toBeNull();
    expect(guardRoute("/admin/u/user-1", true)).toBeNull();
    expect(guardRoute("/admin/edit/u/user-1", true)).toBeNull();
  });

  it("sends the root path to the dashboard or login based on the session", () => {
    expect(guardRoute("/", true)).toEqual({ redirectTo: "/dashboard" });
    expect(guardRoute("/", false)).toEqual({ redirectTo: "/login" });
  });

  it("leaves public paths untouched", () => {
    expect(guardRoute("/login", false)).toBeNull();
    expect(guardRoute("/register", false)).toBeNull();
    expect(guardRoute("/api/auth/me", false)).toBeNull();
  });

  it("exposes the session cookie name the API issues", () => {
    expect(SESSION_COOKIE).toBe("pumapanel_token");
  });
});
