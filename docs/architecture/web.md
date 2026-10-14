# Web App (`apps/web`)

The frontend is a [Next.js](https://nextjs.org) 16 / [React](https://react.dev) 19 application using the App Router, styled with [Tailwind CSS v4](https://tailwindcss.com) and [shadcn/ui](https://ui.shadcn.com) components. It owns the login and registration UI, an authenticated dashboard, and an admin-only area (a central dashboard, user management, and a servers list), and reflects the current session and role.

## Stack

| Concern    | Choice                                                         |
| ---------- | -------------------------------------------------------------- |
| Framework  | Next.js 16.4.0 (App Router, React Compiler enabled)            |
| UI         | React 19.3.0 + `@base-ui/react`, `lucide-react` icons          |
| URL state  | nuqs 2 (`useQueryState`) for search-param-driven UI            |
| Styling    | Tailwind CSS v4 via the `@tailwindcss/turbopack` loader        |
| Components | shadcn/ui (`components.json`, base color `neutral`)            |
| Tests      | Vitest 5 + Testing Library (`jsdom`)                           |
| Scripts    | `pnpm dev` / `build` / `start` / `lint` / `test` / `typecheck` |

## Source layout

```text
apps/web/
  src/
    app/
      layout.tsx             Root layout: fonts, global styles, AuthProvider
      page.tsx               Root route (redirects to /dashboard or /login)
      login/page.tsx         Sign-in page
      register/page.tsx      Registration page
      dashboard/page.tsx     Authenticated dashboard (session-guarded)
      admin/page.tsx         Admin-only central dashboard (hub)
      admin/servers/page.tsx Admin-only servers list (`?action=create`)
      admin/u/[userId]/page.tsx        Admin-only profile view
      admin/edit/u/[userId]/page.tsx   Admin-only profile editor
      globals.css            Tailwind v4 entry + design tokens
    components/
      auth-provider.tsx      Session context: bootstrap, login, register, logout
      auth-form.tsx          Shared login/registration form (validation + errors)
      auth-shell.tsx         Split-screen layout for the auth pages
      dashboard-view.tsx     Presentational dashboard body
      admin-gate.tsx         Role gate for the admin pages
      admin-dashboard-view.tsx  Presentational admin hub (section cards)
      admin-servers-view.tsx Admin servers screen (URL state via nuqs)
      server-list.tsx        Read-only server list + empty state
      create-server-dialog.tsx  Server-creation preview modal (disabled submit)
      admin-user-view.tsx    Admin read-only profile display
      admin-user-edit-form.tsx  Admin profile editor (username, email, role)
      admin-error-card.tsx   Admin-page error/alert state
      page-loading.tsx       Centered spinner for session-aware pages
      site-header.tsx        Shared sticky header (session-aware)
      logo.tsx               Wordmark
      ui/                    shadcn/ui components (button, input, label, dialog, …)
    lib/
      api.ts                 Typed API client + `ApiError` (auth + admin)
      route-guard.ts         Pure access-control rules (shared with the proxy)
      servers.ts             Server list model + placeholder data (no API yet)
      use-admin-user.ts      Loads one user via the admin API (loading/error)
      validate.ts            Shared username/email validators (mirror the API)
      utils.ts               `cn()` helper (clsx + tailwind-merge)
    proxy.ts                 Optimistic route protection (Next.js Proxy)
    __tests__/               Vitest suites + shared helpers
  components.json            shadcn/ui project config
  next.config.ts             Next.js configuration (incl. `/api` rewrite)
  vitest.config.mts          Vitest configuration
  tsconfig.json              Path alias `@/*` → `./src/*`
```

Add shadcn/ui components from the repo root with:

```bash
pnpm dlx shadcn@latest add <component>
```

## Important `next.config.ts` options

```ts
const apiUrl = process.env.API_URL ?? "http://127.0.0.1:3001";

const nextConfig: NextConfig = {
  cacheComponents: true, // React Compiler component caching
  output: "standalone", // minimal server for container images
  partialPrefetching: true, // App Router partial prefetching
  reactCompiler: true, // React Compiler enabled
  rewrites() {
    return [{ destination: `${apiUrl}/:path*`, source: "/api/:path*" }];
  },
  turbopack: {
    rules: { "*.css": { as: "*.css", loaders: ["@tailwindcss/turbopack"] } },
  },
};
```

- **`output: "standalone"`** is what makes the container image slim: the Dockerfile copies just the standalone server, not the whole `node_modules`.
- **`rewrites()`** proxy `/api/*` to the backend so the browser stays same-origin (see below).
- **Turbopack rule** wires Tailwind v4's CSS processing into the build.
- **`API_URL` is read at build time.** Rewrites are compiled into the build output, so set `API_URL` when building the image (`apps/web/Dockerfile` exposes it as a build arg; `compose.yaml` passes `http://api:3001`). In development it defaults to `http://127.0.0.1:3001`, which matches `moon run api:dev`.

## Authentication

The session token lives in an HttpOnly cookie and is **never readable by JavaScript**, so the client tracks the session through the API instead of the cookie:

- **`AuthProvider`** (`src/components/auth-provider.tsx`) wraps the app in `layout.tsx`. On mount it calls `GET /api/auth/me`; a success sets `status: "authenticated"` with the user, and a `401` (or any failure) sets `status: "anonymous"`. The initial `"loading"` state renders identically on the server and the client, avoiding hydration mismatches. A valid session therefore survives page reloads.
- **`useAuth()`** exposes `{ status, user, login, register, logout }`. `login`/`register` call the API and store the returned user; `logout` calls `POST /api/auth/logout` and clears the local state.
- **`/login` and `/register`** share `AuthForm`, which owns input state, client-side validation, in-flight disabling, and error display. Login takes an email-or-username identifier; registration also requires a username (3–30 letters, numbers, or underscores). Both pages render inside `AuthShell` (a split-screen layout) and, on success, call `useAuth()` and redirect to `/dashboard` via `useRouter`; errors are shown inline (field errors) or as an alert (generic errors).
- **`lib/api.ts`** is the single client boundary. It always sends `credentials: "same-origin"` and normalizes failures into `ApiError` (`status`, `code`, `message`, optional `fields`); network failures become `status: 0`, `code: "network_error"`. `AuthUser` carries the account's `role` (`"user"` | `"admin"`), and `adminApi` wraps the two admin endpoints; `adminApi.getUser`/`updateUser` include no special handling — a non-admin session simply receives the backend's `403`.
- **Validation** (`lib/validate.ts`) mirrors the API's username (3–30 letters, numbers, or underscores) and email rules so the form can fail fast; the server still re-validates every edit.

Because every request goes to `/api/*` on the frontend origin, the cookie is sent automatically with no CORS configuration — mirroring the production reverse proxy. See [API architecture](./api.md) for the token contract.

## Routing & access control

The root path has no page of its own — it exists only to send visitors where they belong: signed-in users to `/dashboard`, everyone else to `/login`. `/dashboard` is available only to signed-in users, and the `/admin/*` pages only to administrators. Access control is layered:

- **`src/proxy.ts`** (Next.js 16 renamed Middleware to Proxy) runs before the route renders. It reads the `pumapanel_token` cookie and redirects `/` to `/dashboard` or `/login` (based on cookie presence) and `/dashboard*` / `/admin*` requests to `/login` when the cookie is absent. It is deliberately _optimistic_: it can only observe that a cookie exists, not whether it is valid, so it acts as a fast pre-filter rather than the security boundary.
- **`app/page.tsx`** and **`app/dashboard/page.tsx`** close the remaining gap — a cookie that is present but invalid or expired. They subscribe to `AuthProvider` (which calls `GET /api/auth/me`) and call `redirect(...)` during render once the session is known. `redirect` throws, so it is never wrapped in `try/catch`.
- **`app/admin/*` pages** add a role layer via **`AdminGate`**, which subscribes to the same provider and `redirect`s anonymous visitors to `/login` and signed-in non-admins to `/dashboard`. The pages then fetch the target user through `adminApi.getUser` (`useAdminUser`) and render the profile view or the edit form. With `cacheComponents` enabled, the `useParams<{ userId }>` read suspends during prerendering, so each page wraps it in a `<Suspense>` boundary (the static shell renders the fallback) and keys the content by `userId` so navigation between users resets the loading state.
- **`/admin`** is the central admin dashboard: a hub of section cards linking to servers and the admin's own account. **`/admin/servers`** lists servers with a **create modal** (shadcn/base-ui `Dialog`) whose visibility lives in the `action` search param (`?action=create`), managed with [nuqs](https://nuqs.dev) so the state is shareable and the Back button closes it. The dialog's backdrop blurs the page behind it (`backdrop-blur`, gated behind `supports-backdrop-filter`). The `NuqsAdapter` is mounted on that route (not the root layout) inside a `<Suspense>` boundary, because its provider reads the search params — which suspends during prerendering under `cacheComponents` — so every other page stays a static shell. Server creation is not implemented in the backend yet, so the modal is a preview with a disabled submit button.

None of this is a security boundary by itself: the API's `AdminUser` extractor re-checks the account's _current_ role on every admin request and rejects non-admins with `403` even if they reach the endpoint directly.

The pure decision logic lives in **`lib/route-guard.ts`** (`guardRoute(pathname, hasSession)`), which the proxy imports and the unit tests exercise directly. Authorization is ultimately enforced by the API on every request; the routing layer only decides what to render. See [API architecture](./api.md).

## Testing

```bash
moon run web:test   # vitest run
```

Suites live in `src/__tests__/` and cover the API client (cookie/credentials, error mapping, network failures, admin endpoints), the shared form (validation, in-flight state, field/generic errors), the shared validators, the session provider (mount bootstrap, login, logout), the route guard (including `/admin` and `/admin/servers` paths), the dashboard and admin views (role badge, admin-tools links, role-selector guard, save flow), and the servers screens (empty state, list rows, the disabled create-dialog submit, and the `action=create` modal round-trip through nuqs' testing adapter). Vitest uses `jsdom`, Testing Library, and native Vite tsconfig-path resolution (`resolve.tsconfigPaths`).

## Quality gates

- `moon run web:lint` — Ultracite (Oxlint + Oxfmt) over the project.
- `moon run web:typecheck` — runs `next typegen` (emits route/`LayoutProps` types into `.next/types/`) followed by `tsc --noEmit`.
- `moon run web:test` — Vitest.
- `moon run web:build` — production build (also exercised by CI).

## See also

- [API architecture](./api.md) — the routes this app calls and the token contract.
- [Monorepo & toolchain](./monorepo.md) — moon tasks and the shared lint package.
- [Development: testing](../development/testing.md).
- [Deployment architecture](../deployment/architecture.md) — how the built app is packaged.
