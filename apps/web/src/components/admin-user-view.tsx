import {
  ArrowLeft,
  AtSign,
  CalendarDays,
  Hash,
  Mail,
  Pencil,
  ShieldCheck,
} from "lucide-react";
import Link from "next/link";
import type { ComponentType, ReactNode } from "react";

import { buttonVariants } from "@/components/ui/button";
import { isAdminUser } from "@/lib/api";
import type { AuthUser } from "@/lib/api";
import { cn } from "@/lib/utils";

interface AdminUserViewProps {
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

const initialOf = (name: string): string => name.slice(0, 1).toUpperCase();

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
    <div className="min-w-0">
      <p className="text-muted-foreground text-xs font-medium tracking-wide uppercase">
        {label}
      </p>
      <p className="text-sm break-words">{children}</p>
    </div>
  </div>
);

/**
 * An administrator's read-only view of a user profile.
 */
export const AdminUserView = ({ user }: AdminUserViewProps) => {
  const admin = isAdminUser(user);

  return (
    <div className="mx-auto flex w-full max-w-3xl flex-col gap-6 px-6 py-10">
      <div className="flex items-center justify-between gap-4">
        <Link
          className={cn(
            buttonVariants({ size: "sm", variant: "ghost" }),
            "text-muted-foreground"
          )}
          href="/dashboard"
        >
          <ArrowLeft className="size-4" />
          Dashboard
        </Link>
        <Link
          className={buttonVariants({ size: "sm" })}
          href={`/admin/edit/u/${user.id}`}
        >
          <Pencil className="size-4" />
          Edit user
        </Link>
      </div>

      <div className="border-border bg-card rounded-2xl border p-6 shadow-sm sm:p-8">
        <div className="mb-6 flex items-center gap-4">
          <span className="bg-brand-soft text-brand-strong grid size-12 place-items-center rounded-full text-lg font-semibold">
            {initialOf(user.username)}
          </span>
          <div className="min-w-0">
            <h1 className="truncate text-xl font-semibold tracking-tight">
              {user.username}
            </h1>
            <p className="text-muted-foreground text-sm">{user.email}</p>
          </div>
          <span
            className={cn(
              "ml-auto inline-flex shrink-0 items-center gap-1.5 rounded-full px-2.5 py-1 text-xs font-medium",
              admin
                ? "bg-brand-soft text-brand-strong"
                : "bg-muted text-muted-foreground"
            )}
          >
            <ShieldCheck className="size-3.5" />
            {admin ? "Admin" : "User"}
          </span>
        </div>

        <div className="grid gap-4 sm:grid-cols-2">
          <DetailRow icon={AtSign} label="Username">
            {user.username}
          </DetailRow>
          <DetailRow icon={Mail} label="Email">
            {user.email}
          </DetailRow>
          <DetailRow icon={ShieldCheck} label="Role">
            {admin ? "Admin" : "User"}
          </DetailRow>
          <DetailRow icon={CalendarDays} label="Joined">
            {formatDate(user.createdAt)}
          </DetailRow>
          <div className="sm:col-span-2">
            <DetailRow icon={Hash} label="User ID">
              {user.id}
            </DetailRow>
          </div>
        </div>
      </div>
    </div>
  );
};
