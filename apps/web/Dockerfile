# syntax=docker/dockerfile:1
#
# Build from the repository root so the pnpm workspace lockfile is available:
#   docker build -f apps/web/Dockerfile -t pumapanel/web .
#
# Next.js is built with `output: "standalone"` (see next.config.ts), so the
# runtime image ships only the traced server bundle and its dependencies.

# --- Dependencies + build ----------------------------------------------------
FROM node:26.11.1-bookworm-slim AS builder

ENV PNPM_HOME=/pnpm \
    PATH=/pnpm:$PATH \
    NEXT_TELEMETRY_DISABLED=1 \
    HUSKY=0

RUN npm install --global pnpm@12.12.0

WORKDIR /app

# Install dependencies from manifests only, so this layer is cached until a
# dependency actually changes.
COPY package.json pnpm-lock.yaml pnpm-workspace.yaml ./
COPY apps/web/package.json apps/web/
COPY packages/lint/package.json packages/lint/
RUN pnpm install --frozen-lockfile

# Internal backend URL that the Next.js server proxies `/api/*` requests to.
# `rewrites()` are compiled into the standalone output, so this must be set at
# build time. Compose passes the `api` service DNS name; the default suits
# running the image next to a local backend.
ARG API_URL=http://127.0.0.1:3001
ENV API_URL=${API_URL}

COPY . .
RUN pnpm --filter web build

# --- Runtime -----------------------------------------------------------------
FROM node:26.11.1-bookworm-slim AS runner

ENV NODE_ENV=production \
    NEXT_TELEMETRY_DISABLED=1 \
    HOSTNAME=0.0.0.0 \
    PORT=3000

WORKDIR /app

COPY --from=builder --chown=node:node /app/apps/web/.next/standalone ./
COPY --from=builder --chown=node:node /app/apps/web/.next/static ./apps/web/.next/static
COPY --from=builder --chown=node:node /app/apps/web/public ./apps/web/public

USER node

EXPOSE 3000

HEALTHCHECK --interval=30s --timeout=3s --start-period=10s --retries=3 \
    CMD node -e "fetch('http://127.0.0.1:3000/').then(r => process.exit(r.ok ? 0 : 1)).catch(() => process.exit(1))"

CMD ["node", "apps/web/server.js"]
