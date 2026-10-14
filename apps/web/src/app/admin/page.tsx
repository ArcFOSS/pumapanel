"use client";

import { AdminDashboardView } from "@/components/admin-dashboard-view";
import { AdminGate } from "@/components/admin-gate";
import { useAuth } from "@/components/auth-provider";
import { PageLoading } from "@/components/page-loading";

/**
 * Central admin dashboard.
 *
 * `AdminGate` owns the session and role checks; this inner component reads the
 * signed-in user from context so it can link to the admin's own account.
 */
const AdminOverview = () => {
  const { user } = useAuth();

  if (!user) {
    return <PageLoading message="Checking your session…" />;
  }

  return <AdminDashboardView user={user} />;
};

const AdminPage = () => (
  <AdminGate>
    <AdminOverview />
  </AdminGate>
);

export default AdminPage;
