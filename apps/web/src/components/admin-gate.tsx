"use client";

import { redirect } from "next/navigation";
import type { ReactNode } from "react";

import { useAuth } from "@/components/auth-provider";
import { PageLoading } from "@/components/page-loading";
import { SiteHeader } from "@/components/site-header";
import { isAdminUser } from "@/lib/api";

interface AdminGateProps {
  children: ReactNode;
}

/**
 * Role gate for the admin pages.
 *
 * The proxy already turns away requests without a session cookie, and the API
 * re-authorizes every request with the `AdminUser` extractor, but this
 * component closes the UX gap: while the session resolves it shows a
 * placeholder, then sends visitors without the `admin` role to the dashboard.
 * `redirect` is allowed in a Client Component's render pass (unlike in event
 * handlers).
 */
export const AdminGate = ({ children }: AdminGateProps) => {
  const { status, user } = useAuth();

  if (status === "anonymous") {
    redirect("/login");
  }

  if (status === "authenticated" && user && !isAdminUser(user)) {
    redirect("/dashboard");
  }

  return (
    <div className="flex flex-1 flex-col">
      <SiteHeader />
      {status === "authenticated" && user ? (
        children
      ) : (
        <PageLoading message="Checking your session…" />
      )}
    </div>
  );
};
