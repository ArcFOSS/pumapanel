import { vi } from "vitest";

import type { AuthUser, FieldError } from "@/lib/api";
import type {
  CreateServerInput,
  PaperVersion,
  RuntimeView,
  Server,
  ServersApi,
} from "@/lib/servers";

/** JSON shape the API returns for auth endpoints and error responses. */
export interface JsonPayload {
  error?: {
    code?: string;
    fields?: FieldError[];
    message?: string;
  };
  user?: AuthUser;
}

export const jsonResponse = (body: JsonPayload, status = 200) =>
  Response.json(body, { status });

/**
 * Replaces the global `fetch` with a mock and returns it. Callers should
 * `vi.unstubAllGlobals()` in `afterEach`.
 */
export const setupFetchStub = () => {
  const fetchMock = vi.fn<typeof fetch>();
  vi.stubGlobal("fetch", fetchMock);
  return fetchMock;
};

export const ADMIN_USER: AuthUser = {
  createdAt: "2026-01-01T00:00:00.000Z",
  email: "admin@example.com",
  id: "admin-1",
  role: "admin",
  username: "admin",
};

export const TEST_USER: AuthUser = {
  ...ADMIN_USER,
  email: "user@example.com",
  id: "user-1",
  role: "user",
  username: "testuser",
};

export const SERVER: Server = {
  createdAt: "2026-02-03T00:00:00.000Z",
  hostPort: 25_565,
  id: "server-1",
  javaMin: 21,
  memoryMb: 2048,
  name: "Survival SMP",
  status: "running",
  version: "1.21.8",
};

export const RUNTIME: RuntimeView = {
  available: true,
  hostEnvironment: "bare-metal",
  message: null,
  runtime: "podman",
  version: "6.1.3",
};

export const VERSIONS: PaperVersion[] = [
  { javaMin: 21, support: "SUPPORTED", version: "1.21.8" },
  { javaMin: 17, support: "UNSUPPORTED", version: "1.20.4" },
];

/** Records the calls a {@link fakeServersApi} received. */
export interface ServersApiCalls {
  created: CreateServerInput[];
  removed: { id: string; removeData: boolean }[];
  started: string[];
  stopped: string[];
}

export interface FakeServersApi {
  api: ServersApi;
  calls: ServersApiCalls;
}

/**
 * A faithful in-memory stand-in for the live `serversApi`, used instead of
 * module mocking so tests exercise the components through their real props.
 */
export const fakeServersApi = (
  overrides: Partial<ServersApi> = {},
  servers: Server[] = [SERVER]
): FakeServersApi => {
  const calls: ServersApiCalls = {
    created: [],
    removed: [],
    started: [],
    stopped: [],
  };

  const api: ServersApi = {
    create: (input) => {
      calls.created.push(input);
      return Promise.resolve({ server: { ...SERVER, name: input.name } });
    },
    list: () => Promise.resolve({ runtime: RUNTIME, servers }),
    remove: (id, removeData = false) => {
      calls.removed.push({ id, removeData });
      return Promise.resolve(null);
    },
    start: (id) => {
      calls.started.push(id);
      return Promise.resolve({ server: SERVER });
    },
    stop: (id) => {
      calls.stopped.push(id);
      return Promise.resolve({ server: SERVER });
    },
    versions: () => Promise.resolve({ stale: false, versions: VERSIONS }),
    ...overrides,
  };

  return { api, calls };
};
