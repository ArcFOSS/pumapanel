import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { CreateServerDialog } from "@/components/create-server-dialog";
import { ApiError } from "@/lib/api";
import type { ServersApi } from "@/lib/servers";

import { fakeServersApi, VERSIONS } from "./helpers";

const renderDialog = (
  props: Partial<Parameters<typeof CreateServerDialog>[0]> = {},
  api: ServersApi = fakeServersApi().api
) => {
  const onCreated = vi.fn();
  const onOpenChange = vi.fn();

  render(
    <CreateServerDialog
      api={api}
      onCreated={onCreated}
      onOpenChange={onOpenChange}
      open
      {...props}
    />
  );

  return { onCreated, onOpenChange };
};

/**
 * Picks a version the way a user would: typing filters the list and opens the
 * suggestions, then clicking a suggestion commits it to the field.
 */
const typeVersion = (input: HTMLElement, value: string) => {
  // `inputType` matters: the combobox ignores autofill-shaped changes (no
  // inputType) and only opens its suggestions for real keystrokes.
  fireEvent.input(input, { inputType: "insertText", target: { value } });
};

const chooseVersion = async (value: string) => {
  typeVersion(await screen.findByLabelText("Version"), value);
  fireEvent.click(await screen.findByRole("option", { name: value }));
};

describe("CreateServerDialog", () => {
  it("loads the version catalogue when it opens", async () => {
    const versions = vi.fn(() =>
      Promise.resolve({ stale: false, versions: VERSIONS })
    );
    const { api } = fakeServersApi({ versions });

    renderDialog({}, api);

    expect(screen.getByRole("dialog")).toBeInTheDocument();
    expect(screen.getByLabelText("Name")).toBeInTheDocument();
    expect(versions).toHaveBeenCalledOnce();

    typeVersion(await screen.findByLabelText("Version"), "1");
    expect(
      await screen.findByRole("option", { name: "1.21.8" })
    ).toBeInTheDocument();
    expect(screen.getByRole("option", { name: "1.20.4" })).toBeInTheDocument();
  });

  it("commits the highlighted suggestion when Enter is pressed", async () => {
    const { api, calls } = fakeServersApi();
    renderDialog({}, api);

    fireEvent.change(screen.getByLabelText("Name"), {
      target: { value: "Creative" },
    });
    const version = await screen.findByLabelText("Version");
    typeVersion(version, "1.21.8");
    fireEvent.keyDown(version, { key: "Enter" });
    fireEvent.click(screen.getByRole("button", { name: "Create server" }));

    await waitFor(() => {
      expect(calls.created).toEqual([
        { memoryMb: 2048, name: "Creative", version: "1.21.8" },
      ]);
    });
  });

  it("does not fetch versions while closed", () => {
    const versions = vi.fn(() =>
      Promise.resolve({ stale: false, versions: VERSIONS })
    );
    const { api } = fakeServersApi({ versions });

    renderDialog({ open: false }, api);

    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect(versions).not.toHaveBeenCalled();
  });

  it("creates a server and asks the caller to refresh", async () => {
    const { api, calls } = fakeServersApi();
    const { onCreated, onOpenChange } = renderDialog({}, api);

    fireEvent.change(screen.getByLabelText("Name"), {
      target: { value: "Survival" },
    });
    await chooseVersion("1.21.8");
    fireEvent.click(screen.getByRole("button", { name: "Create server" }));

    await waitFor(() => {
      expect(calls.created).toEqual([
        { memoryMb: 2048, name: "Survival", version: "1.21.8" },
      ]);
    });
    expect(onCreated).toHaveBeenCalledOnce();
    expect(onOpenChange).toHaveBeenCalledWith(false);
  });

  it("validates the form before calling the API", async () => {
    const { api, calls } = fakeServersApi();
    renderDialog({}, api);

    // Wait for the catalogue so the submit button is enabled.
    await waitFor(() => {
      expect(
        screen.getByRole("button", { name: "Create server" })
      ).toBeEnabled();
    });
    fireEvent.click(screen.getByRole("button", { name: "Create server" }));

    expect(
      await screen.findByText("Enter a name for the server.")
    ).toBeInTheDocument();
    expect(screen.getByText("Choose a PaperMC version.")).toBeInTheDocument();
    expect(calls.created).toHaveLength(0);
  });

  it("shows the API's field errors", async () => {
    const { api } = fakeServersApi({
      create: () => {
        throw new ApiError(400, "validation_error", "Invalid input.", [
          { field: "name", message: "That name is already taken." },
        ]);
      },
    });
    renderDialog({}, api);

    fireEvent.change(screen.getByLabelText("Name"), {
      target: { value: "Survival" },
    });
    await chooseVersion("1.21.8");
    fireEvent.click(screen.getByRole("button", { name: "Create server" }));

    expect(
      await screen.findByText("That name is already taken.")
    ).toBeInTheDocument();
  });

  it("asks to close when cancelled", () => {
    const { onOpenChange } = renderDialog();

    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));

    expect(onOpenChange).toHaveBeenCalledWith(false);
  });
});
