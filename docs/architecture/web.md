# Web App (`apps/web`)

The frontend is a [Next.js](https://nextjs.org) 16 / [React](https://react.dev) 19 application using the App Router, styled with [Tailwind CSS v4](https://tailwindcss.com) and [shadcn/ui](https://ui.shadcn.com) components.

## Stack

| Concern    | Choice                                                  |
| ---------- | ------------------------------------------------------- |
| Framework  | Next.js 16.4.0 (App Router, React Compiler enabled)     |
| UI         | React 19.3.0 + `@base-ui/react`, `lucide-react` icons   |
| Styling    | Tailwind CSS v4 via the `@tailwindcss/turbopack` loader |
| Components | shadcn/ui (`components.json`, base color `neutral`)     |
| Scripts    | `pnpm dev` / `build` / `start` / `lint` / `typecheck`   |

## Source layout

```text
apps/web/
  src/
    app/
      layout.tsx      Root layout: Geist fonts, global styles, HTML shell
      page.tsx        Landing page
      globals.css     Tailwind v4 entry + design tokens
    components/
      ui/button.tsx   shadcn/ui button component
    lib/
      utils.ts        `cn()` helper (clsx + tailwind-merge)
  components.json     shadcn/ui project config
  next.config.ts      Next.js configuration
  tsconfig.json       Path alias `@/*` → `./src/*`
```

Add shadcn/ui components from the repo root with:

```bash
pnpm dlx shadcn@latest add <component>
```

## Important `next.config.ts` options

```ts
const nextConfig: NextConfig = {
  cacheComponents: true, // React Compiler component caching
  output: "standalone", // minimal server for container images
  partialPrefetching: true, // App Router partial prefetching
  reactCompiler: true, // React Compiler enabled
  turbopack: {
    rules: { "*.css": { as: "*.css", loaders: ["@tailwindcss/turbopack"] } },
  },
};
```

- **`output: "standalone"`** is what makes the container image slim: the Dockerfile copies just the standalone server, not the whole `node_modules`.
- **Turbopack rule** wires Tailwind v4's CSS processing into the build.
- Note: there are **no `rewrites()` yet**, so in development the web app does not proxy `/api/*` to the Rust backend — call `http://127.0.0.1:3001/health` directly, or route through the reverse proxy in the containerized deployment.

## Quality gates

- `moon run web:lint` — Ultracite (Oxlint + Oxfmt) over the project.
- `moon run web:typecheck` — runs `next typegen` (emits route/`LayoutProps` types into `.next/types/`) followed by `tsc --noEmit`.
- `moon run web:build` — production build (also exercised by CI).

## See also

- [Monorepo & toolchain](./monorepo.md) — moon tasks and the shared lint package.
- [Development: code style](../development/code-style.md).
- [Deployment architecture](../deployment/architecture.md) — how the built app is packaged.
