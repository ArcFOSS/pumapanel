"use client";

import { redirect } from "next/navigation";

import { useAuth } from "@/components/auth-provider";
import { DashboardView } from "@/components/dashboard-view";
import { PageLoading } from "@/components/page-loading";
import { SiteHeader } from "@/components/site-header";

/**
 * Authenticated dashboard.
 *
 * The proxy already turns away requests without a session cookie. This page
 * closes the remaining gap — a cookie that is present but invalid or expired —
 * by asking the API for the current user and redirecting during render when it
 * comes back anonymous. `redirect` is allowed in a Client Component's render
 * pass (unlike in event handlers).
 */
const DashboardPage = () => {
  const { status, user } = useAuth();

  if (status === "anonymous") {
    redirect("/login");
  }

  return (
    <div className="flex flex-1 flex-col">
      <SiteHeader />
      {status === "authenticated" && user ? (
        <DashboardView user={user} />
      ) : (
        <PageLoading message="Checking your session…" />
      )}
    </div>
  );
};

export default DashboardPage;
