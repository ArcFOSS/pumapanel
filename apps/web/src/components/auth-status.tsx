"use client";

import Link from "next/link";

import { useAuth } from "@/components/auth-provider";
import { Button, buttonVariants } from "@/components/ui/button";
import { cn } from "@/lib/utils";

/**
 * Compact reflection of the current session, used on the landing page so the
 * authentication flow is observable end to end. It is intentionally the only
 * piece of account UI beyond the login/registration pages.
 */
export const AuthStatus = () => {
  const { logout, status, user } = useAuth();

  const handleSignOut = () => {
    void logout();
  };

  if (status === "loading") {
    return (
      <p className="text-muted-foreground text-sm">Checking your session…</p>
    );
  }

  if (status === "authenticated" && user) {
    return (
      <div className="flex flex-wrap items-center justify-center gap-3">
        <span className="text-muted-foreground text-sm">
          Signed in as{" "}
          <span className="text-foreground font-medium">{user.email}</span>
        </span>
        <Button onClick={handleSignOut} variant="outline">
          Sign out
        </Button>
      </div>
    );
  }

  return (
    <div className="flex flex-wrap items-center justify-center gap-3">
      <Link className={cn(buttonVariants())} href="/login">
        Sign in
      </Link>
      <Link
        className={cn(buttonVariants({ variant: "outline" }))}
        href="/register"
      >
        Create account
      </Link>
    </div>
  );
};
