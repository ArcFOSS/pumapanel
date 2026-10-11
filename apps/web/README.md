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

## Editing the home page

Edit `src/app/page.tsx` to change the landing page. The page auto-updates during development.
