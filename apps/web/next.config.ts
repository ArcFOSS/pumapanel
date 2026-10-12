import type { NextConfig } from "next";

// The Next.js server proxies browser requests for `/api/*` to the backend, so
// the browser only ever talks to the frontend origin. That keeps the HttpOnly
// session cookie same-origin (no CORS, `SameSite=Lax` applies) and mirrors the
// production reverse proxy, which also routes `/api` to the API with the prefix
// stripped.
//
// `rewrites()` are compiled into the build output, so `API_URL` must be set at
// build time (see apps/web/Dockerfile and compose.yaml). The default targets a
// backend started with `moon run api:dev`.
const apiUrl = process.env.API_URL ?? "http://127.0.0.1:3001";

const nextConfig: NextConfig = {
  /* config options here */
  cacheComponents: true,
  output: "standalone",
  partialPrefetching: true,
  reactCompiler: true,
  rewrites() {
    return [
      {
        destination: `${apiUrl}/:path*`,
        source: "/api/:path*",
      },
    ];
  },
  turbopack: {
    rules: {
      "*.css": {
        as: "*.css",
        loaders: ["@tailwindcss/turbopack"],
      },
    },
  },
};

export default nextConfig;
