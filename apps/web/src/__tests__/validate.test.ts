import { describe, expect, it } from "vitest";

import { isValidEmail, validateEmail, validateUsername } from "@/lib/validate";

describe("isValidEmail", () => {
  it("accepts a typical address", () => {
    expect(isValidEmail("alice@example.com")).toBe(true);
  });

  it("rejects malformed addresses", () => {
    expect(isValidEmail("alice@example")).toBe(false);
    expect(isValidEmail("alice @example.com")).toBe(false);
    expect(isValidEmail("alice@example@com")).toBe(false);
    expect(isValidEmail("@example.com")).toBe(false);
    expect(isValidEmail("alice@.example.com")).toBe(false);
  });
});

describe("validateUsername", () => {
  it("rejects empty values", () => {
    expect(validateUsername("   ")).toBe("Username is required");
  });

  it("rejects out-of-range lengths", () => {
    expect(validateUsername("ab")).toBe(
      "Username must be between 3 and 30 characters"
    );
    expect(validateUsername("a".repeat(31))).toBe(
      "Username must be between 3 and 30 characters"
    );
  });

  it("rejects disallowed characters", () => {
    expect(validateUsername("bad name!")).toBe(
      "Username may only contain letters, numbers, and underscores"
    );
  });

  it("accepts a valid username", () => {
    expect(validateUsername("test_user_1")).toBeUndefined();
  });
});

describe("validateEmail", () => {
  it("rejects empty values", () => {
    expect(validateEmail("  ")).toBe("Email is required");
  });

  it("rejects malformed addresses", () => {
    expect(validateEmail("not-an-email")).toBe("Enter a valid email address");
  });

  it("accepts a valid address and trims surrounding whitespace", () => {
    expect(validateEmail("  alice@example.com  ")).toBeUndefined();
  });
});
