/**
 * Server-management model.
 *
 * Mirrors the shapes returned by the API's `/admin/servers` endpoints (see
 * `apps/api/src/servers/mod.rs`). Kept separate from the client in `lib/api.ts`
 * so components can import the types without pulling in the request layer.
 */

/** Lifecycle state of a server, as the host reports it. */
export type ServerStatus = "running" | "stopped" | "missing" | "unknown";

/** Container runtime the agent drives. */
export type RuntimeName = "docker" | "podman";

/** Where the agent itself runs. */
const HOST_ENVIRONMENTS = ["bare-metal", "docker", "podman"] as const;

export type HostEnvironment = (typeof HOST_ENVIRONMENTS)[number];

/** A server, as the panel's list presents it. */
export interface Server {
  id: string;
  name: string;
  version: string;
  memoryMb: number;
  hostPort: number;
  /** Minimum Java the version needs, when PaperMC reports one. */
  javaMin: number | null;
  status: ServerStatus;
  createdAt: string;
}

/**
 * What the panel can say about the container runtime.
 *
 * `runtime` is `null` when none could be resolved — no agent is configured, or
 * the agent could not be reached — in which case `message` explains why.
 */
export interface RuntimeView {
  runtime: RuntimeName | null;
  available: boolean;
  version: string | null;
  message: string | null;
  hostEnvironment: HostEnvironment | null;
}

/** Response of `GET /admin/servers`. */
export interface ServersResponse {
  servers: Server[];
  runtime: RuntimeView;
}

/** Response of the create/start/stop endpoints. */
export interface ServerResponse {
  server: Server;
}

/** One selectable PaperMC version. */
export interface PaperVersion {
  version: string;
  /** PaperMC's own support status (`SUPPORTED` / `UNSUPPORTED`). */
  support: string;
  javaMin: number | null;
}

/** Response of `GET /admin/servers/versions`. */
export interface VersionsResponse {
  versions: PaperVersion[];
  /** True when PaperMC was unreachable and a cached list was served. */
  stale: boolean;
}

/**
 * The fields a create request carries. The panel fills in the rest: the id, the
 * host port when omitted, and the Java minimum from the version catalogue.
 */
export interface CreateServerInput {
  name: string;
  version: string;
  memoryMb?: number;
  hostPort?: number;
}

/** The value a create/start/stop call resolves to; a delete resolves to `null`. */
export type ServerMutation = ServerResponse | null;

/**
 * The server-management endpoints the UI depends on.
 *
 * Components receive this as a prop and `lib/api`'s `serversApi` is the live
 * implementation, so tests can inject a fake instead of mocking the module.
 */
export interface ServersApi {
  create: (input: CreateServerInput) => Promise<ServerResponse>;
  list: () => Promise<ServersResponse>;
  remove: (id: string, removeData?: boolean) => Promise<null>;
  start: (id: string) => Promise<ServerResponse>;
  stop: (id: string) => Promise<ServerResponse>;
  versions: () => Promise<VersionsResponse>;
}

/** Outcome of a list call: the data, or a message safe to show a user. */
export type ServersResult =
  | { ok: true; data: ServersResponse }
  | { ok: false; message: string };

/** Outcome of a version-catalogue call. */
export type VersionsResult =
  | { ok: true; data: VersionsResponse }
  | { ok: false; message: string };

/** Display label for each status. */
export const STATUS_LABELS: Record<ServerStatus, string> = {
  missing: "Missing",
  running: "Running",
  stopped: "Stopped",
  unknown: "Unknown",
};

/** Longer explanation for each status, announced next to the label. */
export const STATUS_DESCRIPTIONS: Record<ServerStatus, string> = {
  missing: "The host has no container for this server.",
  running: "The container is running.",
  stopped: "The container exists but is stopped.",
  unknown: "The host could not be reached, so the state is unknown.",
};

/** The runtime's display name, or `null` when none was resolved. */
export const runtimeLabel = (runtime: RuntimeView): string | null => {
  if (runtime.runtime === "docker") {
    return "Docker";
  }
  if (runtime.runtime === "podman") {
    return "Podman";
  }
  return null;
};

/** Formats a memory size for display, e.g. `2 GiB` or `768 MiB`. */
export const formatMemory = (memoryMb: number): string => {
  if (memoryMb >= 1024 && memoryMb % 1024 === 0) {
    return `${memoryMb / 1024} GiB`;
  }
  return `${memoryMb} MiB`;
};
