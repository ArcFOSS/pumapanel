import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { DashboardView } from "@/components/dashboard-view";

import { ADMIN_USER, TEST_USER } from "./helpers";

describe("DashboardView", () => {
  it("shows the signed-in user's account information", () => {
    render(<DashboardView user={TEST_USER} />);

    expect(
      screen.getByRole("heading", { name: "Welcome back" })
    ).toBeInTheDocument();
    expect(screen.getAllByText(TEST_USER.email).length).toBeGreaterThan(0);
    expect(screen.getByText(TEST_USER.username)).toBeInTheDocument();
    expect(screen.getByText(TEST_USER.id)).toBeInTheDocument();
    expect(screen.getByText("Account details")).toBeInTheDocument();
    expect(screen.getAllByText("Jan 1, 2026").length).toBeGreaterThan(0);
  });

  it("hides admin tools from regular users", () => {
    render(<DashboardView user={TEST_USER} />);

    expect(
      screen.queryByRole("heading", { name: "Admin tools" })
    ).not.toBeInTheDocument();
  });

  it("shows admin tools with profile links for admins", () => {
    render(<DashboardView user={ADMIN_USER} />);

    expect(
      screen.getByRole("heading", { name: "Admin tools" })
    ).toBeInTheDocument();
    expect(
      screen.getByRole("link", { name: "Open admin dashboard" })
    ).toHaveAttribute("href", "/admin");
    expect(screen.getByRole("link", { name: "Servers" })).toHaveAttribute(
      "href",
      "/admin/servers"
    );
    expect(
      screen.getByRole("link", { name: "Edit your account" })
    ).toHaveAttribute("href", `/admin/edit/u/${ADMIN_USER.id}`);
  });
});
