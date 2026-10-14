"use client";

import { NuqsAdapter } from "nuqs/adapters/next/app";
import { Suspense } from "react";

import { AdminGate } from "@/components/admin-gate";
import { AdminServersView } from "@/components/admin-servers-view";
import { PageLoading } from "@/components/page-loading";

/**
 * Admin-only servers list.
 *
 * `NuqsAdapter` is mounted here rather than in the root layout because its
 * provider reads the search params, which suspends during prerendering under
 * `cacheComponents`. Scoping it to this route (inside a `<Suspense>` boundary)
 * keeps every other page prerenderable as a static shell.
 */
const AdminServersPage = () => (
  <AdminGate>
    <Suspense fallback={<PageLoading message="Loading servers…" />}>
      <NuqsAdapter>
        <AdminServersView />
      </NuqsAdapter>
    </Suspense>
  </AdminGate>
);

export default AdminServersPage;
