import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { AdminDashboardView } from "@/components/admin-dashboard-view";

import { ADMIN_USER } from "./helpers";

describe("AdminDashboardView", () => {
  it("links to the admin sections", () => {
    render(<AdminDashboardView user={ADMIN_USER} />);

    expect(
      screen.getByRole("heading", { name: "Administration" })
    ).toBeInTheDocument();
    expect(
      screen.getByText(
        `Signed in as ${ADMIN_USER.username}. Manage the panel from here.`
      )
    ).toBeInTheDocument();
    expect(screen.getByRole("link", { name: /^Servers/u })).toHaveAttribute(
      "href",
      "/admin/servers"
    );
    expect(
      screen.getByRole("link", { name: /^Your account/u })
    ).toHaveAttribute("href", `/admin/u/${ADMIN_USER.id}`);
  });
});
