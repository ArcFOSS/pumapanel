import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { AdminUserView } from "@/components/admin-user-view";

import { ADMIN_USER, TEST_USER } from "./helpers";

describe("AdminUserView", () => {
  it("renders a user's profile with navigation links", () => {
    render(<AdminUserView user={TEST_USER} />);

    expect(
      screen.getByRole("heading", { name: TEST_USER.username })
    ).toBeInTheDocument();
    expect(screen.getAllByText(TEST_USER.email).length).toBeGreaterThan(0);
    expect(screen.getByText("User ID")).toBeInTheDocument();
    expect(screen.getByText(TEST_USER.id)).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "Dashboard" })).toHaveAttribute(
      "href",
      "/dashboard"
    );
    expect(screen.getByRole("link", { name: "Edit user" })).toHaveAttribute(
      "href",
      `/admin/edit/u/${TEST_USER.id}`
    );
  });

  it("shows the role for user and admin accounts", () => {
    const { rerender } = render(<AdminUserView user={TEST_USER} />);

    expect(screen.getAllByText("User").length).toBeGreaterThan(0);
    expect(screen.queryByText("Admin")).not.toBeInTheDocument();

    rerender(<AdminUserView user={ADMIN_USER} />);
    expect(screen.getAllByText("Admin").length).toBeGreaterThan(0);
  });
});
