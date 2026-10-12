"use client";

import { useParams } from "next/navigation";
import { Suspense } from "react";

import { AdminErrorCard } from "@/components/admin-error-card";
import { AdminGate } from "@/components/admin-gate";
import { AdminUserEditForm } from "@/components/admin-user-edit-form";
import { useAuth } from "@/components/auth-provider";
import { PageLoading } from "@/components/page-loading";
import { useAdminUser } from "@/lib/use-admin-user";

/**
 * Admin-only page for editing a user's permitted details.
 */
const AdminEditUser = ({ userId }: { userId: string }) => {
  const { user: actingUser } = useAuth();
  const state = useAdminUser(userId);

  if (state.status === "loading") {
    return <PageLoading message="Loading user…" />;
  }
  if (state.status === "error") {
    return <AdminErrorCard message={state.message} />;
  }
  return (
    <AdminUserEditForm isSelf={actingUser?.id === userId} user={state.user} />
  );
};

const AdminEditUserId = () => {
  const { userId } = useParams<{ userId: string }>();

  // Keying by id remounts the subtree when the segment changes, so the loading
  // state resets and the form never shows another user's draft.
  return <AdminEditUser key={userId} userId={userId} />;
};

/**
 * Like the view page, the `useParams` read sits under a `<Suspense>` boundary
 * so the route's static shell can be prerendered under `cacheComponents`.
 */
const AdminEditUserPage = () => (
  <AdminGate>
    <Suspense fallback={<PageLoading message="Loading user…" />}>
      <AdminEditUserId />
    </Suspense>
  </AdminGate>
);

export default AdminEditUserPage;
