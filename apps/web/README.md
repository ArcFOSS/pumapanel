# PumaPanel Web

The Next.js frontend for PumaPanel. See the [repository README](../../README.md) for the full monorepo setup.

## Development

Run commands from the repository root through [moon](https://moonrepo.dev):

```bash
moon run web:dev        # dev server on http://localhost:3000
moon run web:build      # production build
moon run web:lint       # lint + format check (Ultracite)
moon run web:typecheck  # type-check with tsc
```

The app uses:

- [Next.js](https://nextjs.org) App Router
- [Tailwind CSS v4](https://tailwindcss.com)
- [shadcn/ui](https://ui.shadcn.com) — add components with `pnpm dlx shadcn@latest add <component>`

## Routing

There is no standalone home page. The root route (`src/app/page.tsx`) redirects to `/dashboard` when a session is present and `/login` otherwise, and the dashboard (`src/app/dashboard/page.tsx`) is guarded the same way. Edit those files to change the authenticated experience.
