import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { withNuqsTestingAdapter } from "nuqs/adapters/testing";
import { describe, expect, it, vi } from "vitest";

import { AdminServersView } from "@/components/admin-servers-view";
import { ApiError } from "@/lib/api";
import type { RuntimeView, ServersApi } from "@/lib/servers";

import { fakeServersApi, RUNTIME, SERVER } from "./helpers";

const renderView = (api: ServersApi = fakeServersApi().api) =>
  render(<AdminServersView api={api} />, {
    wrapper: withNuqsTestingAdapter({ searchParams: "" }),
  });

describe("AdminServersView", () => {
  it("loads the server list and reports the runtime", async () => {
    renderView();

    expect(await screen.findByText("Survival SMP")).toBeInTheDocument();
    expect(screen.getByText("Podman 6.1.3")).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Create server" })
    ).toBeInTheDocument();
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("shows an empty state when there are no servers", async () => {
    renderView(fakeServersApi({}, []).api);

    expect(await screen.findByText("No servers yet")).toBeInTheDocument();
  });

  it("explains why the controls are inert when no runtime is available", async () => {
    const unavailable: RuntimeView = {
      available: false,
      hostEnvironment: null,
      message: "No agent is configured.",
      runtime: null,
      version: null,
    };
    renderView(
      fakeServersApi({
        list: () =>
          Promise.resolve({ runtime: unavailable, servers: [SERVER] }),
      }).api
    );

    expect(
      await screen.findByText("No agent is configured.")
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Stop Survival SMP" })
    ).toBeDisabled();
  });

  it("opens the create dialog when the URL already asks for it", async () => {
    render(<AdminServersView api={fakeServersApi().api} />, {
      wrapper: withNuqsTestingAdapter({ searchParams: "?action=create" }),
    });

    expect(await screen.findByRole("dialog")).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { name: "Create a server" })
    ).toBeInTheDocument();
  });

  it("reflects the create action in the URL when the button is clicked", async () => {
    const onUrlUpdate = vi.fn();
    render(<AdminServersView api={fakeServersApi().api} />, {
      wrapper: withNuqsTestingAdapter({ onUrlUpdate }),
    });

    fireEvent.click(
      await screen.findByRole("button", { name: "Create server" })
    );

    expect(
      await screen.findByRole("heading", { name: "Create a server" })
    ).toBeInTheDocument();
    await waitFor(() => {
      expect(onUrlUpdate).toHaveBeenCalledOnce();
    });
    expect(onUrlUpdate.mock.calls[0]?.[0].queryString).toBe("?action=create");
  });

  it("clears the action from the URL when the dialog is cancelled", async () => {
    const onUrlUpdate = vi.fn();
    render(<AdminServersView api={fakeServersApi().api} />, {
      wrapper: withNuqsTestingAdapter({
        onUrlUpdate,
        searchParams: "?action=create",
      }),
    });

    fireEvent.click(await screen.findByRole("button", { name: "Cancel" }));

    await waitFor(() => {
      expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    });
    expect(onUrlUpdate).toHaveBeenCalledOnce();
    expect(onUrlUpdate.mock.calls[0]?.[0].queryString).toBe("");
  });

  it("stops a running server and reloads the list", async () => {
    const { api, calls } = fakeServersApi();
    renderView(api);

    fireEvent.click(
      await screen.findByRole("button", { name: "Stop Survival SMP" })
    );

    await waitFor(() => {
      expect(calls.stopped).toEqual(["server-1"]);
    });
  });

  it("surfaces a list failure and offers a retry", async () => {
    const list = vi
      .fn<ServersApi["list"]>()
      .mockRejectedValueOnce(new ApiError(500, "internal_error", "Boom."))
      .mockResolvedValue({ runtime: RUNTIME, servers: [SERVER] });
    renderView(fakeServersApi({ list }).api);

    expect(await screen.findByText("Boom.")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Retry" }));

    expect(await screen.findByText("Survival SMP")).toBeInTheDocument();
  });
});
