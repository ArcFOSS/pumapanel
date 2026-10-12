/**
 * Route protection rules shared by the edge proxy and the client-side guards.
 *
 * The proxy runs before a request is rendered and can only observe whether a
 * session cookie is *present* — it cannot verify it. These checks are therefore
 * optimistic: the API stays the authority on whether a session is valid, and it
 * authorizes every request regardless of what the UI decides to render. Admin
 * pages additionally check the (client-known) role and, more importantly, the
 * backend re-authorizes every admin request with the `AdminUser` extractor.
 */

/** Name of the HttpOnly session cookie issued by the API. */
export const SESSION_COOKIE = "pumapanel_token";

/**
 * Paths that require an authenticated session. `/admin` is included so that
 * visitors without a cookie are turned away before the (admin-only) page code
 * runs; the role check happens in the admin gate and inside the API.
 */
const PROTECTED_PREFIXES = ["/dashboard", "/admin"] as const;

export interface RouteGuardDecision {
  readonly redirectTo: string;
}

const isProtectedPath = (pathname: string): boolean =>
  PROTECTED_PREFIXES.some(
    (prefix) => pathname === prefix || pathname.startsWith(`${prefix}/`)
  );

/**
 * Returns the redirect a request should receive, or `null` to let it render.
 *
 * The root path has no page of its own: it sends authenticated visitors to the
 * dashboard and everyone else to the login screen. Any case the proxy cannot
 * resolve (for example an expired cookie) is settled by the destination page.
 */
export const guardRoute = (
  pathname: string,
  hasSession: boolean
): RouteGuardDecision | null => {
  if (pathname === "/") {
    return { redirectTo: hasSession ? "/dashboard" : "/login" };
  }
  if (!hasSession && isProtectedPath(pathname)) {
    return { redirectTo: "/login" };
  }
  return null;
};
