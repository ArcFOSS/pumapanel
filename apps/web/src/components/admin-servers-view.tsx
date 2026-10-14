"use client";

import { Plus } from "lucide-react";
import { parseAsStringLiteral, useQueryState } from "nuqs";
import { useCallback, useEffect, useState } from "react";

import { CreateServerDialog } from "@/components/create-server-dialog";
import { PageLoading } from "@/components/page-loading";
import { ServerList } from "@/components/server-list";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  ApiError,
  GENERIC_ERROR_MESSAGE,
  loadServers,
  serversApi,
} from "@/lib/api";
import type {
  RuntimeView,
  Server,
  ServerMutation,
  ServersApi,
  ServersResult,
} from "@/lib/servers";
import { runtimeLabel } from "@/lib/servers";

/** Accepted values for the `action` search param on the servers page. */
const SERVER_ACTIONS = ["create"] as const;

/**
 * `action` lives in the URL (via nuqs) instead of component state, so the
 * create dialog is shareable and the Back button closes it. `history: "push"`
 * makes opening the dialog a navigation-style event, per the nuqs guidance for
 * modal-like UI.
 */
const actionParser = parseAsStringLiteral(SERVER_ACTIONS).withOptions({
  history: "push",
});

/** Shown until the first list response arrives. */
const CHECKING_RUNTIME: RuntimeView = {
  available: false,
  hostEnvironment: null,
  message: "Checking the container runtime…",
  runtime: null,
  version: null,
};

interface AdminServersViewProps {
  /** Defaults to the live API client; injectable so tests can supply a fake. */
  api?: ServersApi;
}

/**
 * Describes the container runtime the panel is talking to.
 *
 * When none is available the caller renders the reason separately, so this
 * stays short and never overflows the header.
 */
const RuntimeBadge = ({ runtime }: { runtime: RuntimeView }) => {
  const label = runtimeLabel(runtime);

  if (!runtime.available || !label) {
    return (
      <span className="border-destructive/30 bg-destructive/10 text-destructive inline-flex shrink-0 items-center rounded-full border px-2.5 py-1 text-xs font-medium">
        Runtime unavailable
      </span>
    );
  }

  const version = runtime.version ? ` ${runtime.version}` : "";

  return (
    <span className="border-border text-muted-foreground inline-flex shrink-0 items-center rounded-full border px-2.5 py-1 text-xs font-medium">
      {`${label}${version}`}
    </span>
  );
};

/**
 * Admin servers screen.
 *
 * Loads the server list and the host's runtime state from the panel, then
 * renders the header, the URL-controlled create dialog, and the list. Mutations
 * re-read the list afterwards so the rows reflect the host rather than an
 * optimistic guess.
 *
 * `api` is injectable so tests can pass a fake implementation; the page uses the
 * live client.
 */
export const AdminServersView = ({
  api = serversApi,
}: AdminServersViewProps) => {
  const [action, setAction] = useQueryState("action", actionParser);
  const creating = action === "create";

  const [servers, setServers] = useState<Server[] | null>(null);
  const [runtime, setRuntime] = useState<RuntimeView>(CHECKING_RUNTIME);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [pendingId, setPendingId] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [pendingDelete, setPendingDelete] = useState<Server | null>(null);

  // Applies a list response to state. Shared by the mount effect, the Retry
  // button, and every mutation so those paths cannot drift.
  const applyList = useCallback((result: ServersResult) => {
    if (result.ok) {
      setServers(result.data.servers);
      setRuntime(result.data.runtime);
      setLoadError(null);
    } else {
      setLoadError(result.message);
    }
  }, []);

  const reload = useCallback(async () => {
    applyList(await loadServers(api));
  }, [api, applyList]);

  // Load on mount, and again if the injected client changes.
  useEffect(() => {
    let active = true;
    const load = async () => {
      const result = await loadServers(api);
      if (active) {
        applyList(result);
      }
    };

    void load();

    return () => {
      active = false;
    };
  }, [api, applyList]);

  const runAction = async (id: string, run: () => Promise<ServerMutation>) => {
    setPendingId(id);
    setNotice(null);
    try {
      await run();
      await reload();
    } catch (error) {
      setNotice(
        error instanceof ApiError ? error.message : GENERIC_ERROR_MESSAGE
      );
    }
    setPendingId(null);
  };

  const handleCreatingChange = (open: boolean) => {
    void setAction(open ? "create" : null);
  };

  const handleCreated = () => {
    void reload();
  };

  const handleRetry = () => {
    void reload();
  };

  const handleStart = (server: Server) => {
    void runAction(server.id, () => api.start(server.id));
  };

  const handleStop = (server: Server) => {
    void runAction(server.id, () => api.stop(server.id));
  };

  const handleRequestDelete = (server: Server) => {
    setPendingDelete(server);
  };

  const handleDeleteDialogChange = (next: boolean) => {
    if (!next) {
      setPendingDelete(null);
    }
  };

  const handleConfirmDelete = () => {
    const server = pendingDelete;
    if (!server) {
      return;
    }
    setPendingDelete(null);
    void runAction(server.id, () => api.remove(server.id, true));
  };

  return (
    <div className="mx-auto flex w-full max-w-5xl flex-col gap-6 px-6 py-10">
      <div className="flex flex-wrap items-start justify-between gap-4">
        <div className="flex flex-col gap-1">
          <h1 className="text-2xl font-semibold tracking-tight">Servers</h1>
          <p className="text-muted-foreground text-sm">
            Every server registered on the panel.
          </p>
        </div>
        <div className="flex flex-wrap items-center gap-3">
          <RuntimeBadge runtime={runtime} />
          <Button onClick={() => handleCreatingChange(true)} type="button">
            <Plus className="size-4" />
            Create server
          </Button>
        </div>
      </div>

      {runtime.message && !runtime.available ? (
        <p className="border-destructive/30 bg-destructive/5 text-destructive rounded-2xl border p-4 text-sm">
          {runtime.message}
        </p>
      ) : null}

      <CreateServerDialog
        api={api}
        onCreated={handleCreated}
        onOpenChange={handleCreatingChange}
        open={creating}
      />

      {notice ? (
        <p className="text-destructive text-sm" role="alert">
          {notice}
        </p>
      ) : null}

      {loadError ? (
        <div className="border-destructive/30 bg-destructive/5 flex flex-wrap items-center justify-between gap-3 rounded-2xl border p-4">
          <p className="text-destructive text-sm" role="alert">
            {loadError}
          </p>
          <Button
            onClick={handleRetry}
            size="sm"
            type="button"
            variant="outline"
          >
            Retry
          </Button>
        </div>
      ) : null}

      {servers === null && !loadError ? (
        <PageLoading message="Loading servers…" />
      ) : null}

      {servers === null ? null : (
        <ServerList
          onDelete={handleRequestDelete}
          onStart={handleStart}
          onStop={handleStop}
          pendingId={pendingId}
          runtime={runtime}
          servers={servers}
        />
      )}

      <Dialog
        onOpenChange={handleDeleteDialogChange}
        open={pendingDelete !== null}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Delete this server?</DialogTitle>
            <DialogDescription>
              {`“${pendingDelete?.name ?? ""}” and its world data will be removed `}
              from the host. This cannot be undone.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button
              onClick={() => handleDeleteDialogChange(false)}
              type="button"
              variant="outline"
            >
              Cancel
            </Button>
            <Button
              disabled={pendingId !== null}
              onClick={handleConfirmDelete}
              type="button"
              variant="destructive"
            >
              Delete server
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
};
