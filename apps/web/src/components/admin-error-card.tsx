import { ArrowLeft, TriangleAlert } from "lucide-react";
import Link from "next/link";

import { buttonVariants } from "@/components/ui/button";
import { cn } from "@/lib/utils";

interface AdminErrorCardProps {
  message: string;
}

/**
 * Error state for the admin pages: an alert plus a way back to the dashboard.
 * Used when loading a user fails (for example a `403`, `404`, or network
 * error).
 */
export const AdminErrorCard = ({ message }: AdminErrorCardProps) => (
  <div className="flex flex-1 items-center justify-center p-10">
    <div className="border-border bg-card flex max-w-md flex-col items-center gap-4 rounded-2xl border p-8 text-center shadow-sm">
      <span className="bg-destructive/10 text-destructive grid size-11 place-items-center rounded-full">
        <TriangleAlert className="size-5" />
      </span>
      <p className="text-sm" role="alert">
        {message}
      </p>
      <Link
        className={cn(buttonVariants({ size: "sm", variant: "outline" }))}
        href="/dashboard"
      >
        <ArrowLeft className="size-4" />
        Back to dashboard
      </Link>
    </div>
  </div>
);
