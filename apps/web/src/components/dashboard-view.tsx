import {
  AtSign,
  BadgeCheck,
  CalendarDays,
  Hash,
  KeyRound,
  Mail,
  ShieldCheck,
} from "lucide-react";
import Link from "next/link";
import type { ComponentType, ReactNode } from "react";

import { buttonVariants } from "@/components/ui/button";
import { isAdminUser } from "@/lib/api";
import type { AuthUser } from "@/lib/api";
import { cn } from "@/lib/utils";

interface DashboardViewProps {
  user: AuthUser;
}

const DATE_FORMATTER = new Intl.DateTimeFormat("en-US", {
  dateStyle: "medium",
});

const formatDate = (value: string): string => {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return value;
  }
  return DATE_FORMATTER.format(date);
};

const initialOf = (email: string): string => email.slice(0, 1).toUpperCase();

interface DetailRowProps {
  icon: ComponentType<{ className?: string }>;
  label: string;
  children: ReactNode;
}

const DetailRow = ({ icon: Icon, label, children }: DetailRowProps) => (
  <div className="flex items-start gap-3">
    <span className="bg-brand-soft text-brand-strong grid size-9 shrink-0 place-items-center rounded-lg">
      <Icon className="size-4" />
    </span>
    <div className="flex min-w-0 flex-col gap-0.5">
      <span className="text-muted-foreground text-xs font-medium tracking-wide uppercase">
        {label}
      </span>
      <span className="text-foreground truncate text-sm font-medium">
        {children}
      </span>
    </div>
  </div>
);

interface StatCardProps {
  icon: ComponentType<{ className?: string }>;
  label: string;
  value: string;
}

const StatCard = ({ icon: Icon, label, value }: StatCardProps) => (
  <div
    className={cn(
      "border-border bg-card flex items-center gap-3 rounded-xl border p-4 shadow-sm"
    )}
  >
    <span className="bg-brand text-brand-foreground grid size-10 shrink-0 place-items-center rounded-lg">
      <Icon className="size-5" />
    </span>
    <div className="flex flex-col">
      <span className="text-muted-foreground text-xs">{label}</span>
      <span className="text-foreground text-sm font-semibold">{value}</span>
    </div>
  </div>
);

/**
 * Presentational dashboard body.
 *
 * Kept free of routing and data fetching so it can be rendered in isolation;
 * the page owns the session guard and supplies the signed-in user.
 */
export const DashboardView = ({ user }: DashboardViewProps) => {
  const admin = isAdminUser(user);

  return (
    <div className="mx-auto flex w-full max-w-5xl flex-col gap-8 px-6 py-10">
      <div className="flex items-center gap-4">
        <span className="from-brand to-brand-strong text-brand-foreground grid size-12 place-items-center rounded-full bg-linear-to-br text-lg font-semibold shadow-sm">
          {initialOf(user.email)}
        </span>
        <div className="flex flex-col">
          <h1 className="text-2xl font-semibold tracking-tight">
            Welcome back
          </h1>
          <p className="text-muted-foreground text-sm">{user.email}</p>
        </div>
      </div>

      <div className="grid gap-4 sm:grid-cols-3">
        <StatCard icon={BadgeCheck} label="Account status" value="Active" />
        <StatCard icon={KeyRound} label="Session type" value="JWT · HttpOnly" />
        <StatCard
          icon={CalendarDays}
          label="Member since"
          value={formatDate(user.createdAt)}
        />
      </div>

      <section className="border-border bg-card flex flex-col gap-6 rounded-2xl border p-6 shadow-sm">
        <div className="flex flex-col gap-1">
          <h2 className="text-lg font-semibold tracking-tight">
            Account details
          </h2>
          <p className="text-muted-foreground text-sm">
            Read-only information from your authenticated session.
          </p>
        </div>
        <div className="grid gap-5 sm:grid-cols-2">
          <DetailRow icon={AtSign} label="Username">
            {user.username}
          </DetailRow>
          <DetailRow icon={Mail} label="Email">
            {user.email}
          </DetailRow>
          <DetailRow icon={ShieldCheck} label="Role">
            {admin ? "Admin" : "User"}
          </DetailRow>
          <DetailRow icon={Hash} label="User ID">
            {user.id}
          </DetailRow>
          <DetailRow icon={CalendarDays} label="Created">
            {formatDate(user.createdAt)}
          </DetailRow>
          <DetailRow icon={KeyRound} label="Authentication">
            Email &amp; password
          </DetailRow>
        </div>
      </section>

      {admin ? (
        <section className="border-brand/30 bg-brand-soft/40 flex flex-col gap-3 rounded-2xl border border-dashed p-6">
          <div className="flex flex-col gap-1">
            <h2 className="text-base font-semibold tracking-tight">
              Admin tools
            </h2>
            <p className="text-muted-foreground text-sm">
              Manage servers and user accounts. These pages are only reachable
              by administrators, and the API re-checks the role on every
              request.
            </p>
          </div>
          <div className="flex flex-wrap items-center gap-2">
            <Link
              className={buttonVariants({ size: "sm", variant: "outline" })}
              href="/admin"
            >
              Open admin dashboard
            </Link>
            <Link
              className={buttonVariants({ size: "sm", variant: "outline" })}
              href="/admin/servers"
            >
              Servers
            </Link>
            <Link
              className={buttonVariants({ size: "sm", variant: "outline" })}
              href={`/admin/edit/u/${user.id}`}
            >
              Edit your account
            </Link>
          </div>
        </section>
      ) : null}

      <section className="border-brand/30 bg-brand-soft/40 flex flex-col gap-2 rounded-2xl border border-dashed p-6">
        <h2 className="text-base font-semibold tracking-tight">
          You&apos;re all set
        </h2>
        <p className="text-muted-foreground text-sm">
          This dashboard is only reachable with a valid session. Unauthenticated
          visitors are redirected to the login page before it renders.
        </p>
      </section>
    </div>
  );
};
