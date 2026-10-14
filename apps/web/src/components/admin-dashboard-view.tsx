import { ArrowRight, Server, ShieldCheck } from "lucide-react";
import Link from "next/link";
import type { ComponentType } from "react";

import type { AuthUser } from "@/lib/api";

interface AdminDashboardViewProps {
  user: AuthUser;
}

interface AdminSectionCardProps {
  description: string;
  href: string;
  icon: ComponentType<{ className?: string }>;
  title: string;
}

const AdminSectionCard = ({
  description,
  href,
  icon: Icon,
  title,
}: AdminSectionCardProps) => (
  <Link
    className="border-border bg-card hover:border-brand/40 hover:bg-brand-soft/30 group flex flex-col gap-4 rounded-2xl border p-6 shadow-sm transition-colors"
    href={href}
  >
    <span className="bg-brand text-brand-foreground grid size-10 place-items-center rounded-lg">
      <Icon className="size-5" />
    </span>
    <span className="flex flex-col gap-1">
      <span className="flex items-center gap-1.5 text-base font-semibold tracking-tight">
        {title}
        <ArrowRight className="size-4 transition-transform group-hover:translate-x-0.5" />
      </span>
      <span className="text-muted-foreground text-sm">{description}</span>
    </span>
  </Link>
);

/**
 * Central admin dashboard: an index of the administration surfaces.
 *
 * Presentational and free of routing/data fetching so it can be rendered in
 * isolation; the page owns the admin gate and supplies the signed-in user.
 */
export const AdminDashboardView = ({ user }: AdminDashboardViewProps) => (
  <div className="mx-auto flex w-full max-w-5xl flex-col gap-8 px-6 py-10">
    <div className="flex flex-col gap-1">
      <h1 className="text-2xl font-semibold tracking-tight">Administration</h1>
      <p className="text-muted-foreground text-sm">
        Signed in as {user.username}. Manage the panel from here.
      </p>
    </div>

    <div className="grid gap-4 sm:grid-cols-2">
      <AdminSectionCard
        description="Browse every server on the panel. Creating servers is coming soon."
        href="/admin/servers"
        icon={Server}
        title="Servers"
      />
      <AdminSectionCard
        description="Review and edit your own administrator account."
        href={`/admin/u/${user.id}`}
        icon={ShieldCheck}
        title="Your account"
      />
    </div>
  </div>
);
