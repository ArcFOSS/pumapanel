import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { AuthForm } from "@/components/auth-form";
import { ApiError } from "@/lib/api";

const renderLoginForm = (
  onSubmit: (identifier: string, password: string) => Promise<void>
) =>
  render(
    <AuthForm
      description="Sign in with your email or username."
      footer={null}
      onSubmit={onSubmit}
      submitLabel="Sign in"
      title="Welcome back"
      variant="login"
    />
  );

const renderRegisterForm = (
  onSubmit: (username: string, email: string, password: string) => Promise<void>
) =>
  render(
    <AuthForm
      description="Create an account."
      footer={null}
      onSubmit={onSubmit}
      passwordMinLength={8}
      submitLabel="Create account"
      title="Create your account"
      variant="register"
    />
  );

const fillLogin = (identifier: string, password: string) => {
  fireEvent.change(screen.getByLabelText("Email or username"), {
    target: { value: identifier },
  });
  fireEvent.change(screen.getByLabelText("Password"), {
    target: { value: password },
  });
};

const fillRegister = (username: string, email: string, password: string) => {
  fireEvent.change(screen.getByLabelText("Username"), {
    target: { value: username },
  });
  fireEvent.change(screen.getByLabelText("Email"), {
    target: { value: email },
  });
  fireEvent.change(screen.getByLabelText("Password"), {
    target: { value: password },
  });
};

describe("AuthForm login", () => {
  it("renders a heading and fields associated with their labels", () => {
    renderLoginForm(() => Promise.resolve());

    expect(
      screen.getByRole("heading", { name: "Welcome back" })
    ).toBeInTheDocument();
    expect(screen.getByLabelText("Email or username")).toBeInTheDocument();
    expect(screen.getByLabelText("Password")).toBeInTheDocument();
  });

  it("validates required fields without calling the API", async () => {
    const onSubmit = vi.fn(() => Promise.resolve());
    renderLoginForm(onSubmit);

    fireEvent.click(screen.getByRole("button", { name: "Sign in" }));

    expect(
      await screen.findByText("Email or username is required")
    ).toBeInTheDocument();
    expect(screen.getByText("Password is required")).toBeInTheDocument();
    expect(onSubmit).not.toHaveBeenCalled();
  });

  it("submits an email identifier", async () => {
    const onSubmit = vi.fn(() => Promise.resolve());
    renderLoginForm(onSubmit);

    fillLogin("user@example.com", "password123");
    fireEvent.click(screen.getByRole("button", { name: "Sign in" }));

    await waitFor(() => {
      expect(onSubmit).toHaveBeenCalledWith("user@example.com", "password123");
    });
  });

  it("submits a username identifier without requiring an email", async () => {
    const onSubmit = vi.fn(() => Promise.resolve());
    renderLoginForm(onSubmit);

    fillLogin("testuser", "password123");
    fireEvent.click(screen.getByRole("button", { name: "Sign in" }));

    await waitFor(() => {
      expect(onSubmit).toHaveBeenCalledWith("testuser", "password123");
    });
  });

  it("disables the submit button while the request is in flight", () => {
    const onSubmit = vi.fn(() => Promise.resolve());
    renderLoginForm(onSubmit);

    fillLogin("user@example.com", "password123");
    const button = screen.getByRole("button", { name: "Sign in" });
    fireEvent.click(button);

    expect(button).toBeDisabled();
    expect(button).toHaveTextContent("Please wait…");
  });

  it("toggles password visibility with the eye button", () => {
    renderLoginForm(() => Promise.resolve());

    const password = screen.getByLabelText("Password");
    expect(password).toHaveAttribute("type", "password");

    fireEvent.click(screen.getByRole("button", { name: "Show password" }));
    expect(password).toHaveAttribute("type", "text");

    fireEvent.click(screen.getByRole("button", { name: "Hide password" }));
    expect(password).toHaveAttribute("type", "password");
  });

  it("renders a generic API error as an alert", async () => {
    const onSubmit = vi.fn(() =>
      Promise.reject(
        new ApiError(
          401,
          "invalid_credentials",
          "Invalid email, username, or password."
        )
      )
    );
    renderLoginForm(onSubmit);

    fillLogin("user@example.com", "wrong-password");
    fireEvent.click(screen.getByRole("button", { name: "Sign in" }));

    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Invalid email, username, or password."
    );
  });
});

describe("AuthForm register", () => {
  it("renders username, email, and password fields", () => {
    renderRegisterForm(() => Promise.resolve());

    expect(screen.getByLabelText("Username")).toBeInTheDocument();
    expect(screen.getByLabelText("Email")).toBeInTheDocument();
    expect(screen.getByLabelText("Password")).toBeInTheDocument();
  });

  it.each([
    {
      email: "new@example.com",
      message: "Username is required",
      password: "password123",
      title: "requires a username",
      username: "",
    },
    {
      email: "new@example.com",
      message: "Username may only contain letters, numbers, and underscores",
      password: "password123",
      title: "rejects a malformed username",
      username: "bad name!",
    },
    {
      email: "new@example.com",
      message: "Password must be at least 8 characters",
      password: "short",
      title: "enforces the minimum password length",
      username: "newuser",
    },
  ])("$title", async ({ email, message, password, username }) => {
    const onSubmit = vi.fn(() => Promise.resolve());
    renderRegisterForm(onSubmit);

    fillRegister(username, email, password);
    fireEvent.click(screen.getByRole("button", { name: "Create account" }));

    expect(await screen.findByText(message)).toBeInTheDocument();
    expect(onSubmit).not.toHaveBeenCalled();
  });

  it("submits the username, email, and password", async () => {
    const onSubmit = vi.fn(() => Promise.resolve());
    renderRegisterForm(onSubmit);

    fillRegister("newuser", "new@example.com", "password123");
    fireEvent.click(screen.getByRole("button", { name: "Create account" }));

    await waitFor(() => {
      expect(onSubmit).toHaveBeenCalledWith(
        "newuser",
        "new@example.com",
        "password123"
      );
    });
  });

  it("renders field errors returned by the API", async () => {
    const onSubmit = vi.fn(() =>
      Promise.reject(
        new ApiError(409, "username_taken", "That username is taken.", [
          { field: "username", message: "That username is taken." },
        ])
      )
    );
    renderRegisterForm(onSubmit);

    fillRegister("taken", "new@example.com", "password123");
    fireEvent.click(screen.getByRole("button", { name: "Create account" }));

    expect(
      await screen.findByText("That username is taken.")
    ).toBeInTheDocument();
  });
});
