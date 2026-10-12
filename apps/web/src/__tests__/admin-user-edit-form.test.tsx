import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { AdminUserEditForm } from "@/components/admin-user-edit-form";

import { jsonResponse, setupFetchStub, TEST_USER } from "./helpers";

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("AdminUserEditForm", () => {
  it("disables the role selector for the account's own profile", () => {
    render(<AdminUserEditForm isSelf={true} user={TEST_USER} />);

    expect(screen.getByLabelText("Role")).toBeDisabled();
    expect(
      screen.getByText("You cannot change your own role.")
    ).toBeInTheDocument();
  });

  it("lets an admin change other users' roles", () => {
    render(<AdminUserEditForm isSelf={false} user={TEST_USER} />);

    expect(screen.getByLabelText("Role")).not.toBeDisabled();
  });

  it("validates fields client-side before submitting", async () => {
    render(<AdminUserEditForm isSelf={false} user={TEST_USER} />);

    fireEvent.change(screen.getByLabelText("Username"), {
      target: { value: "ab" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save changes" }));

    expect(
      await screen.findByText("Username must be between 3 and 30 characters")
    ).toBeInTheDocument();
  });

  it("submits a valid edit and reports success", async () => {
    const fetchMock = setupFetchStub();
    fetchMock.mockResolvedValue(
      jsonResponse({ user: { ...TEST_USER, username: "renamed" } })
    );
    render(<AdminUserEditForm isSelf={false} user={TEST_USER} />);

    fireEvent.change(screen.getByLabelText("Username"), {
      target: { value: "renamed" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save changes" }));

    await waitFor(() => {
      expect(fetchMock).toHaveBeenCalledTimes(1);
    });
    const [[url, init]] = fetchMock.mock.calls;
    expect(url).toBe("/api/admin/users/user-1");
    expect(init?.method).toBe("PATCH");
    expect(init?.body).toBe(
      JSON.stringify({
        email: TEST_USER.email,
        role: TEST_USER.role,
        username: "renamed",
      })
    );

    expect(await screen.findByText("User updated.")).toBeInTheDocument();
  });

  it("submits a role change for another user", async () => {
    const fetchMock = setupFetchStub();
    fetchMock.mockResolvedValue(
      jsonResponse({ user: { ...TEST_USER, role: "admin" } })
    );
    render(<AdminUserEditForm isSelf={false} user={TEST_USER} />);

    fireEvent.change(screen.getByLabelText("Role"), {
      target: { value: "admin" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save changes" }));

    await waitFor(() => {
      expect(fetchMock).toHaveBeenCalledTimes(1);
    });
    const [[, init]] = fetchMock.mock.calls;
    expect(init?.body).toBe(
      JSON.stringify({
        email: TEST_USER.email,
        role: "admin",
        username: TEST_USER.username,
      })
    );
  });

  it("maps server field errors onto the fields", async () => {
    const fetchMock = setupFetchStub();
    fetchMock.mockResolvedValue(
      jsonResponse(
        {
          error: {
            code: "validation_error",
            fields: [
              { field: "email", message: "Enter a valid email address" },
            ],
            message: "Invalid input",
          },
        },
        400
      )
    );
    render(<AdminUserEditForm isSelf={false} user={TEST_USER} />);

    fireEvent.click(screen.getByRole("button", { name: "Save changes" }));

    expect(
      await screen.findByText("Enter a valid email address")
    ).toBeInTheDocument();
  });
});
