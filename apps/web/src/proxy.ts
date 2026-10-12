import { NextResponse } from "next/server";
import type { NextRequest } from "next/server";

import { guardRoute, SESSION_COOKIE } from "@/lib/route-guard";

/**
 * Optimistic route protection.
 *
 * This runs before a route is rendered and redirects visitors without a session
 * cookie away from protected paths (the dashboard and the admin area). Because
 * the cookie is HttpOnly and signed by the API, the proxy cannot verify it
 * here — the dashboard confirms the session with the API once it loads, the
 * admin pages additionally confirm the account's role, and the API enforces
 * authorization on every request. Treat this as a fast pre-filter, not the
 * security boundary.
 */
export const proxy = (request: NextRequest) => {
  const hasSession = Boolean(request.cookies.get(SESSION_COOKIE)?.value);
  const decision = guardRoute(request.nextUrl.pathname, hasSession);

  if (decision) {
    return NextResponse.redirect(new URL(decision.redirectTo, request.url));
  }

  return NextResponse.next();
};

export const config = {
  matcher: ["/", "/dashboard", "/dashboard/:path*", "/admin", "/admin/:path*"],
};
