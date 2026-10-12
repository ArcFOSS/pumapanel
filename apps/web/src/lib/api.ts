/**
 * Typed client for the backend API.
 *
 * Requests target `/api/*` on the current origin; the Next.js server proxies
 * them to the backend (see `next.config.ts`). Because everything stays
 * same-origin, the HttpOnly session cookie is sent automatically and no CORS
 * configuration is required. The cookie is never read by JavaScript.
 */

export type UserRole = "user" | "admin";

/** Returns true when the account holds the `admin` role. */
export const isAdminUser = (user: Pick<AuthUser, "role">): boolean =>
  user.role === "admin";

export interface AuthUser {
  id: string;
  email: string;
  username: string;
  role: UserRole;
  createdAt: string;
}

export interface AuthResponse {
  user: AuthUser;
}

export interface FieldError {
  field: string;
  message: string;
}

interface ApiErrorBody {
  error?: {
    code?: string;
    message?: string;
    fields?: FieldError[];
  };
}

/**
 * Error thrown for non-2xx responses (and for network failures, with status 0).
 * `fields` is populated for `validation_error` responses.
 */
export class ApiError extends Error {
  readonly status: number;
  readonly code: string;
  readonly fields?: FieldError[];

  constructor(
    status: number,
    code: string,
    message: string,
    fields?: FieldError[]
  ) {
    super(message);
    this.name = "ApiError";
    this.status = status;
    this.code = code;
    this.fields = fields;
  }
}

const NETWORK_ERROR_MESSAGE =
  "Unable to reach the server. Check your connection and try again.";

const send = async (path: string, init?: RequestInit): Promise<Response> => {
  try {
    return await fetch(`/api${path}`, {
      ...init,
      credentials: "same-origin",
      headers: {
        "Content-Type": "application/json",
        ...init?.headers,
      },
    });
  } catch {
    throw new ApiError(0, "network_error", NETWORK_ERROR_MESSAGE);
  }
};

const readJson = async (response: Response): Promise<ApiErrorBody | null> => {
  const text = await response.text();
  if (text.length === 0) {
    return null;
  }
  try {
    // SAFETY: the API responds with a JSON object; a body that does not parse
    // is discarded and surfaced as a generic error by the caller.
    return JSON.parse(text) as ApiErrorBody;
  } catch {
    return null;
  }
};

const request = async <T>(path: string, init?: RequestInit): Promise<T> => {
  const response = await send(path, init);
  const body = await readJson(response);

  if (!response.ok) {
    throw new ApiError(
      response.status,
      body?.error?.code ?? "unknown_error",
      body?.error?.message ?? "Something went wrong. Please try again.",
      body?.error?.fields
    );
  }

  // SAFETY: each endpoint pins its response contract at the call site; the
  // backend returns `{ user }` for the authenticated and admin user endpoints
  // and an empty body (parsed as `null`) for logout.
  return body as T;
};

export const authApi = {
  login: (identifier: string, password: string) =>
    request<AuthResponse>("/auth/login", {
      body: JSON.stringify({ identifier, password }),
      method: "POST",
    }),

  logout: () => request<null>("/auth/logout", { method: "POST" }),

  me: () => request<AuthResponse>("/auth/me"),

  register: (username: string, email: string, password: string) =>
    request<AuthResponse>("/auth/register", {
      body: JSON.stringify({ email, password, username }),
      method: "POST",
    }),
};

/** The fields an admin may change on a user account. */
export interface UpdateUserInput {
  email?: string;
  role?: UserRole;
  username?: string;
}

/**
 * Client for the admin user-management endpoints. Every call is authorized by
 * the backend's `AdminUser` extractor; a non-admin session receives `403`.
 */
export const adminApi = {
  getUser: (userId: string) => request<AuthResponse>(`/admin/users/${userId}`),

  updateUser: (userId: string, changes: UpdateUserInput) =>
    request<AuthResponse>(`/admin/users/${userId}`, {
      body: JSON.stringify(changes),
      method: "PATCH",
    }),
};
