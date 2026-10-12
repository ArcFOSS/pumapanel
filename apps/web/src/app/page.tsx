"use client";

import { redirect } from "next/navigation";

import { useAuth } from "@/components/auth-provider";
import { PageLoading } from "@/components/page-loading";

/**
 * Root route.
 *
 * There is no marketing page: visitors with a session go to the dashboard and
 * everyone else goes to the login screen. The proxy resolves this from the
 * cookie before render; this component is the accurate fallback, redirecting
 * from the real session that the auth provider loads from the API.
 */
const Home = () => {
  const { status } = useAuth();

  if (status === "loading") {
    return <PageLoading message="Redirecting…" />;
  }

  redirect(status === "authenticated" ? "/dashboard" : "/login");
};

export default Home;
