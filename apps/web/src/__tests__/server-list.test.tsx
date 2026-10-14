import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { ServerList } from "@/components/server-list";
import type { RuntimeView, Server } from "@/lib/servers";

const RUNTIME: RuntimeView = {
  available: true,
  hostEnvironment: "bare-metal",
  message: null,
  runtime: "podman",
  version: "6.1.3",
};

const SERVERS: Server[] = [
  {
    createdAt: "2026-02-03T00:00:00.000Z",
    hostPort: 25_565,
    id: "server-1",
    javaMin: 21,
    memoryMb: 2048,
    name: "Survival SMP",
    status: "running",
    version: "1.21.8",
  },
  {
    createdAt: "2026-01-15T00:00:00.000Z",
    hostPort: 25_566,
    id: "server-2",
    javaMin: 21,
    memoryMb: 4096,
    name: "Creative Plot",
    status: "stopped",
    version: "1.21.8",
  },
];

const renderList = (runtime: RuntimeView = RUNTIME) => {
  const handleDelete = vi.fn();
  const handleStart = vi.fn();
  const handleStop = vi.fn();

  render(
    <ServerList
      onDelete={handleDelete}
      onStart={handleStart}
      onStop={handleStop}
      pendingId={null}
      runtime={runtime}
      servers={SERVERS}
    />
  );

  return { handleDelete, handleStart, handleStop };
};

describe("ServerList", () => {
  it("shows an empty state when there are no servers", () => {
    render(
      <ServerList
        onDelete={vi.fn()}
        onStart={vi.fn()}
        onStop={vi.fn()}
        pendingId={null}
        runtime={RUNTIME}
        servers={[]}
      />
    );

    expect(screen.getByText("No servers yet")).toBeInTheDocument();
    expect(screen.queryByRole("list")).not.toBeInTheDocument();
  });

  it("lists each server with its version, port, memory, and status", () => {
    renderList();

    expect(screen.getAllByRole("listitem")).toHaveLength(SERVERS.length);
    expect(screen.getByText("Survival SMP")).toBeInTheDocument();
    expect(screen.getByText("Creative Plot")).toBeInTheDocument();
    expect(screen.getAllByText(/Paper 1\.21\.8/u)).toHaveLength(SERVERS.length);
    expect(screen.getByText(/Port 25565/u)).toBeInTheDocument();
    expect(screen.getByText(/Port 25566/u)).toBeInTheDocument();
    expect(screen.getByText(/2 GiB/u)).toBeInTheDocument();
    expect(screen.getByText(/4 GiB/u)).toBeInTheDocument();
    expect(screen.getByText("Running")).toBeInTheDocument();
    expect(screen.getByText("Stopped")).toBeInTheDocument();
  });

  it("stops a running server and starts a stopped one", () => {
    const handlers = renderList();

    fireEvent.click(screen.getByRole("button", { name: "Stop Survival SMP" }));
    expect(handlers.handleStop).toHaveBeenCalledWith(SERVERS[0]);
    expect(handlers.handleStart).not.toHaveBeenCalled();

    fireEvent.click(
      screen.getByRole("button", { name: "Start Creative Plot" })
    );
    expect(handlers.handleStart).toHaveBeenCalledWith(SERVERS[1]);
  });

  it("asks to delete a server", () => {
    const handlers = renderList();

    fireEvent.click(
      screen.getByRole("button", { name: "Delete Survival SMP" })
    );

    expect(handlers.handleDelete).toHaveBeenCalledWith(SERVERS[0]);
  });

  it("disables the controls when no runtime is available", () => {
    renderList({ ...RUNTIME, available: false, message: "No agent." });

    expect(
      screen.getByRole("button", { name: "Stop Survival SMP" })
    ).toBeDisabled();
    expect(
      screen.getByRole("button", { name: "Start Creative Plot" })
    ).toBeDisabled();
    // Deleting a record is still possible without a runtime.
    expect(
      screen.getByRole("button", { name: "Delete Survival SMP" })
    ).toBeEnabled();
  });
});
