import { Check, PawPrint } from "lucide-react";
import Link from "next/link";
import type { ReactNode } from "react";

const HIGHLIGHTS = [
  "Argon2id password hashing",
  "HttpOnly cookie sessions",
  "Typed Rust + TypeScript stack",
] as const;

interface AuthShellProps {
  children: ReactNode;
}

/**
 * Split-screen layout shared by the login and registration pages: a branded
 * panel on large screens, and the supplied form centered on small ones.
 */
export const AuthShell = ({ children }: AuthShellProps) => (
  <div className="bg-background grid min-h-svh lg:grid-cols-2">
    <aside className="from-brand-deep to-brand-deeper relative hidden flex-col justify-between overflow-hidden bg-linear-to-br p-10 text-white lg:flex">
      <div
        aria-hidden="true"
        className="absolute -top-24 -right-24 size-72 rounded-full bg-white/10 blur-3xl"
      />
      <div
        aria-hidden="true"
        className="absolute -bottom-32 -left-16 size-80 rounded-full bg-black/20 blur-3xl"
      />

      <Link
        className="focus-visible:ring-ring/50 relative z-10 inline-flex items-center gap-2 rounded-lg outline-none focus-visible:ring-3"
        href="/"
      >
        <span className="grid size-8 place-items-center rounded-lg bg-white/15 backdrop-blur">
          <PawPrint className="size-4" />
        </span>
        <span className="text-base font-semibold tracking-tight">
          PumaPanel
        </span>
      </Link>

      <div className="relative z-10 flex flex-col gap-6">
        <h2 className="max-w-sm text-3xl font-semibold tracking-tight">
          A production-minded start for your next panel.
        </h2>
        <ul className="flex flex-col gap-3 text-sm text-white/90">
          {HIGHLIGHTS.map((item) => (
            <li className="flex items-center gap-2" key={item}>
              <Check className="size-4" />
              {item}
            </li>
          ))}
        </ul>
      </div>

      <p className="relative z-10 text-xs text-white/70">
        Sessions are signed and stored in an HttpOnly cookie.
      </p>
    </aside>

    <main className="flex items-center justify-center p-6">{children}</main>
  </div>
);
