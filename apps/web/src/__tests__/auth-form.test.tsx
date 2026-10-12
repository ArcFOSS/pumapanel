import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { AuthForm } from "@/components/auth-form";
import { ApiError } from "@/lib/api";

const renderLoginForm = (
  onSubmit: (email: string, password: string) => Promise<void>
) =>
  render(
    <AuthForm
      description="Sign in to your account."
      footer={null}
      onSubmit={onSubmit}
      passwordAutoComplete="current-password"
      submitLabel="Sign in"
      title="Welcome back"
    />
  );

const fillCredentials = (email: string, password: string) => {
  fireEvent.change(screen.getByLabelText("Email"), {
    target: { value: email },
  });
  fireEvent.change(screen.getByLabelText("Password"), {
    target: { value: password },
  });
};

describe("AuthForm", () => {
  it("renders a heading and fields associated with their labels", () => {
    renderLoginForm(() => Promise.resolve());

    expect(
      screen.getByRole("heading", { name: "Welcome back" })
    ).toBeInTheDocument();
    expect(screen.getByLabelText("Email")).toBeInTheDocument();
    expect(screen.getByLabelText("Password")).toBeInTheDocument();
  });

  it("validates required fields without calling the API", async () => {
    const onSubmit = vi.fn(() => Promise.resolve());
    renderLoginForm(onSubmit);

    fireEvent.click(screen.getByRole("button", { name: "Sign in" }));

    expect(await screen.findByText("Email is required")).toBeInTheDocument();
    expect(screen.getByText("Password is required")).toBeInTheDocument();
    expect(onSubmit).not.toHaveBeenCalled();
  });

  it("rejects a malformed email before submitting", async () => {
    const onSubmit = vi.fn(() => Promise.resolve());
    renderLoginForm(onSubmit);

    fillCredentials("not-an-email", "password123");
    fireEvent.click(screen.getByRole("button", { name: "Sign in" }));

    expect(
      await screen.findByText("Enter a valid email address")
    ).toBeInTheDocument();
    expect(onSubmit).not.toHaveBeenCalled();
  });

  it("submits the entered credentials", async () => {
    const onSubmit = vi.fn(() => Promise.resolve());
    renderLoginForm(onSubmit);

    fillCredentials("user@example.com", "password123");
    fireEvent.click(screen.getByRole("button", { name: "Sign in" }));

    await waitFor(() => {
      expect(onSubmit).toHaveBeenCalledWith("user@example.com", "password123");
    });
  });

  it("disables the submit button while the request is in flight", () => {
    const onSubmit = vi.fn(() => Promise.resolve());
    renderLoginForm(onSubmit);

    fillCredentials("user@example.com", "password123");
    const button = screen.getByRole("button", { name: "Sign in" });
    fireEvent.click(button);

    expect(button).toBeDisabled();
    expect(button).toHaveTextContent("Please wait…");
  });

  it("renders field errors returned by the API", async () => {
    const onSubmit = vi.fn(() =>
      Promise.reject(
        new ApiError(409, "email_taken", "That email is already registered.", [
          { field: "email", message: "That email is already registered." },
        ])
      )
    );
    renderLoginForm(onSubmit);

    fillCredentials("taken@example.com", "password123");
    fireEvent.click(screen.getByRole("button", { name: "Sign in" }));

    expect(
      await screen.findByText("That email is already registered.")
    ).toBeInTheDocument();
  });

  it("renders a generic API error as an alert", async () => {
    const onSubmit = vi.fn(() =>
      Promise.reject(
        new ApiError(401, "invalid_credentials", "Invalid email or password.")
      )
    );
    renderLoginForm(onSubmit);

    fillCredentials("user@example.com", "wrong-password");
    fireEvent.click(screen.getByRole("button", { name: "Sign in" }));

    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Invalid email or password."
    );
  });

  it("enforces the minimum password length during registration", async () => {
    const onSubmit = vi.fn(() => Promise.resolve());
    render(
      <AuthForm
        description="Create an account."
        footer={null}
        onSubmit={onSubmit}
        passwordAutoComplete="new-password"
        passwordMinLength={8}
        submitLabel="Create account"
        title="Create your account"
      />
    );

    fillCredentials("new@example.com", "short");
    fireEvent.click(screen.getByRole("button", { name: "Create account" }));

    expect(
      await screen.findByText("Password must be at least 8 characters")
    ).toBeInTheDocument();
    expect(onSubmit).not.toHaveBeenCalled();
  });
});
