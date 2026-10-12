"use client";

import { useParams } from "next/navigation";
import { Suspense } from "react";

import { AdminErrorCard } from "@/components/admin-error-card";
import { AdminGate } from "@/components/admin-gate";
import { AdminUserView } from "@/components/admin-user-view";
import { PageLoading } from "@/components/page-loading";
import { useAdminUser } from "@/lib/use-admin-user";

/**
 * Admin-only view of a user's profile.
 */
const AdminUser = ({ userId }: { userId: string }) => {
  const state = useAdminUser(userId);

  if (state.status === "loading") {
    return <PageLoading message="Loading user…" />;
  }
  if (state.status === "error") {
    return <AdminErrorCard message={state.message} />;
  }
  return <AdminUserView user={state.user} />;
};

const AdminUserId = () => {
  const { userId } = useParams<{ userId: string }>();

  // Keying by id remounts the subtree when the segment changes (an admin
  // navigating from one user to another), so the loading state resets instead
  // of flashing the previous user's data.
  return <AdminUser key={userId} userId={userId} />;
};

/**
 * With `cacheComponents` enabled, reading the dynamic `userId` param suspends
 * during prerendering, so the `useParams` call lives in a leaf component under
 * a `<Suspense>` boundary; the static shell renders the fallback instead of
 * failing the build.
 */
const AdminUserPage = () => (
  <AdminGate>
    <Suspense fallback={<PageLoading message="Loading user…" />}>
      <AdminUserId />
    </Suspense>
  </AdminGate>
);

export default AdminUserPage;
