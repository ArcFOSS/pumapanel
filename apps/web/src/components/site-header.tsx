"use client";

import { LogOut, Shield, UserPlus } from "lucide-react";
import Link from "next/link";

import { useAuth } from "@/components/auth-provider";
import { Logo } from "@/components/logo";
import { Button, buttonVariants } from "@/components/ui/button";
import { isAdminUser } from "@/lib/api";
import { cn } from "@/lib/utils";

/**
 * Sticky top navigation shared by the authenticated surfaces.
 *
 * The right-hand actions reflect the client session: a fixed placeholder while
 * the session is loading (identical on the server, so hydration matches), then
 * "Sign in / Get started", or for signed-in users "Sign out" plus an "Admin"
 * link when the account holds the `admin` role.
 */
export const SiteHeader = () => {
  const { logout, status, user } = useAuth();

  const handleSignOut = () => {
    // Failing to clear the server cookie must not block signing out locally;
    // the dashboard guard redirects once the session state flips to anonymous.
    void logout();
  };

  return (
    <header className="bg-background/80 sticky top-0 z-50 border-b backdrop-blur-md">
      <div className="mx-auto flex h-14 max-w-6xl items-center justify-between gap-4 px-6">
        <Logo />

        <div className="flex items-center gap-2">
          {status === "loading" ? (
            <span
              aria-hidden="true"
              className="bg-muted h-8 w-24 animate-pulse rounded-lg"
            />
          ) : null}

          {status === "authenticated" ? (
            <>
              {user && isAdminUser(user) ? (
                <Link
                  className={cn(
                    buttonVariants({ size: "sm", variant: "ghost" }),
                    "hidden sm:inline-flex"
                  )}
                  href={`/admin/u/${user.id}`}
                >
                  <Shield className="size-4" />
                  Admin
                </Link>
              ) : null}
              <Button
                onClick={handleSignOut}
                size="sm"
                type="button"
                variant="outline"
              >
                <LogOut className="size-4" />
                Sign out
              </Button>
            </>
          ) : null}

          {status === "anonymous" ? (
            <>
              <Link
                className={cn(
                  buttonVariants({ size: "sm", variant: "ghost" }),
                  "hidden sm:inline-flex"
                )}
                href="/login"
              >
                Sign in
              </Link>
              <Link className={buttonVariants({ size: "sm" })} href="/register">
                <UserPlus className="size-4" />
                Get started
              </Link>
            </>
          ) : null}
        </div>
      </div>
    </header>
  );
};
