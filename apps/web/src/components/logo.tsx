import { PawPrint } from "lucide-react";
import Link from "next/link";

import { cn } from "@/lib/utils";

interface LogoProps {
  className?: string;
}

/** Wordmark linking home, used in the site header and the auth shell. */
export const Logo = ({ className }: LogoProps) => (
  <Link
    className={cn(
      "group focus-visible:ring-ring/50 inline-flex items-center gap-2 rounded-lg outline-none focus-visible:ring-3",
      className
    )}
    href="/"
  >
    <span className="from-brand to-brand-strong text-brand-foreground grid size-8 place-items-center rounded-lg bg-linear-to-br shadow-sm transition-transform group-hover:scale-105">
      <PawPrint className="size-4" />
    </span>
    <span className="text-base font-semibold tracking-tight">PumaPanel</span>
  </Link>
);
