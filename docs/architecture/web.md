# Web App (`apps/web`)

The frontend is a [Next.js](https://nextjs.org) 16 / [React](https://react.dev) 19 application using the App Router, styled with [Tailwind CSS v4](https://tailwindcss.com) and [shadcn/ui](https://ui.shadcn.com) components. It owns the login and registration UI and reflects the current session.

## Stack

| Concern    | Choice                                                         |
| ---------- | -------------------------------------------------------------- |
| Framework  | Next.js 16.4.0 (App Router, React Compiler enabled)            |
| UI         | React 19.3.0 + `@base-ui/react`, `lucide-react` icons          |
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
      page.tsx               Landing page (+ session status)
      login/page.tsx         Sign-in page
      register/page.tsx      Registration page
      globals.css            Tailwind v4 entry + design tokens
    components/
      auth-provider.tsx      Session context: bootstrap, login, register, logout
      auth-form.tsx          Shared email/password form (validation + errors)
      auth-status.tsx        Landing-page session summary
      ui/                    shadcn/ui components (button, input, label, …)
    lib/
      api.ts                 Typed API client + `ApiError`
      utils.ts               `cn()` helper (clsx + tailwind-merge)
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
- **`/login` and `/register`** share `AuthForm`, which owns input state, client-side validation, in-flight disabling, and error display. Both pages call `useAuth()` and redirect to `/` on success via `useRouter`; errors are shown inline (field errors) or as an alert (generic errors).
- **`lib/api.ts`** is the single client boundary. It always sends `credentials: "same-origin"` and normalizes failures into `ApiError` (`status`, `code`, `message`, optional `fields`); network failures become `status: 0`, `code: "network_error"`.

Because every request goes to `/api/*` on the frontend origin, the cookie is sent automatically with no CORS configuration — mirroring the production reverse proxy. See [API architecture](./api.md) for the token contract.

## Testing

```bash
moon run web:test   # vitest run
```

Suites live in `src/__tests__/` and cover the API client (cookie/credentials, error mapping, network failures), the shared form (validation, in-flight state, field/generic errors), and the session provider (mount bootstrap, login, logout). Vitest uses `jsdom`, Testing Library, and native Vite tsconfig-path resolution (`resolve.tsconfigPaths`).

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
