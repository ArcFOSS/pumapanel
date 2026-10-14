"use client";

import {
  Loader2,
  Play,
  Server as ServerIcon,
  Square,
  Trash2,
} from "lucide-react";
import type { ReactNode } from "react";

import { Button } from "@/components/ui/button";
import type { RuntimeView, Server, ServerStatus } from "@/lib/servers";
import {
  formatMemory,
  STATUS_DESCRIPTIONS,
  STATUS_LABELS,
} from "@/lib/servers";
import { cn } from "@/lib/utils";

interface ServerListProps {
  /** The server whose action is in flight, if any. */
  pendingId: string | null;
  /** Asks the caller to confirm and remove a server. */
  onDelete: (server: Server) => void;
  onStart: (server: Server) => void;
  onStop: (server: Server) => void;
  runtime: RuntimeView;
  servers: readonly Server[];
}

const DATE_FORMATTER = new Intl.DateTimeFormat("en-US", {
  dateStyle: "medium",
});

const formatDate = (value: string): string => {
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? value : DATE_FORMATTER.format(date);
};

const STATUS_CLASSES = {
  missing: "bg-destructive/10 text-destructive",
  running: "bg-brand-soft text-brand-strong",
  stopped: "bg-muted text-muted-foreground",
  unknown: "border-border text-muted-foreground border",
} satisfies Record<ServerStatus, string>;

const Meta = ({ children }: { children: ReactNode }) => (
  <span className="text-muted-foreground text-xs">{children}</span>
);

const EmptyState = () => (
  <div className="border-border bg-card flex flex-col items-center gap-3 rounded-2xl border border-dashed p-10 text-center">
    <span className="bg-brand-soft text-brand-strong grid size-11 place-items-center rounded-full">
      <ServerIcon className="size-5" />
    </span>
    <div className="flex flex-col gap-1">
      <p className="text-sm font-medium">No servers yet</p>
      <p className="text-muted-foreground text-sm">
        Create your first PaperMC server to see it here.
      </p>
    </div>
  </div>
);

/**
 * The server list: each row shows the panel's record plus the host's live
 * state, with the controls that act on it.
 *
 * Presentational — the caller owns the data, the in-flight id, and the
 * confirmation flow, so this can be rendered in isolation in tests.
 */
export const ServerList = ({
  onDelete,
  onStart,
  onStop,
  pendingId,
  runtime,
  servers,
}: ServerListProps) => {
  if (servers.length === 0) {
    return <EmptyState />;
  }

  // Without a reachable runtime the controls cannot work, so they stay
  // disabled rather than failing after a click.
  const actionsDisabled = !runtime.available;

  return (
    <ul className="border-border bg-card divide-border divide-y overflow-hidden rounded-2xl border shadow-sm">
      {servers.map((server) => {
        const pending = pendingId === server.id;
        const running = server.status === "running";

        return (
          <li
            className="flex flex-wrap items-center justify-between gap-4 p-4"
            key={server.id}
          >
            <div className="flex min-w-0 flex-col gap-1">
              <div className="flex items-center gap-2">
                <span className="truncate text-sm font-medium">
                  {server.name}
                </span>
                <span
                  className={cn(
                    "inline-flex shrink-0 items-center rounded-full px-2.5 py-1 text-xs font-medium",
                    STATUS_CLASSES[server.status]
                  )}
                >
                  {STATUS_LABELS[server.status]}
                  <span className="sr-only">
                    : {STATUS_DESCRIPTIONS[server.status]}
                  </span>
                </span>
              </div>
              <div className="flex flex-wrap items-center gap-x-3 gap-y-0.5">
                <Meta>Paper {server.version}</Meta>
                <Meta>Port {server.hostPort}</Meta>
                <Meta>{formatMemory(server.memoryMb)}</Meta>
                <Meta>Created {formatDate(server.createdAt)}</Meta>
              </div>
            </div>

            <div className="flex items-center gap-1.5">
              {running ? (
                <Button
                  aria-label={`Stop ${server.name}`}
                  disabled={pending || actionsDisabled}
                  onClick={() => onStop(server)}
                  size="sm"
                  type="button"
                  variant="outline"
                >
                  {pending ? (
                    <Loader2 className="size-4 animate-spin" />
                  ) : (
                    <Square className="size-4" />
                  )}
                  Stop
                </Button>
              ) : (
                <Button
                  aria-label={`Start ${server.name}`}
                  disabled={pending || actionsDisabled}
                  onClick={() => onStart(server)}
                  size="sm"
                  type="button"
                  variant="outline"
                >
                  {pending ? (
                    <Loader2 className="size-4 animate-spin" />
                  ) : (
                    <Play className="size-4" />
                  )}
                  Start
                </Button>
              )}
              <Button
                aria-label={`Delete ${server.name}`}
                disabled={pending}
                onClick={() => onDelete(server)}
                size="sm"
                type="button"
                variant="ghost"
              >
                <Trash2 className="size-4" />
                Delete
              </Button>
            </div>
          </li>
        );
      })}
    </ul>
  );
};
