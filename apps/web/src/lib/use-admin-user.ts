import { useEffect, useState } from "react";

import { ApiError, adminApi } from "@/lib/api";
import type { AuthUser } from "@/lib/api";

export type AdminUserState =
  | { status: "loading" }
  | { status: "error"; message: string }
  | { status: "success"; user: AuthUser };

/**
 * Loads one user via the admin API, tracking loading and error states.
 *
 * The admin pages remount this hook per user id (with a `key`), so each mount
 * starts from the initial `loading` state; the effect only updates state after
 * the request settles.
 */
export const useAdminUser = (userId: string): AdminUserState => {
  const [state, setState] = useState<AdminUserState>({ status: "loading" });

  useEffect(() => {
    let active = true;

    const load = async () => {
      try {
        const { user } = await adminApi.getUser(userId);
        if (active) {
          setState({ status: "success", user });
        }
      } catch (error) {
        if (active) {
          const message =
            error instanceof ApiError
              ? error.message
              : "Unable to load this user.";
          setState({ message, status: "error" });
        }
      }
    };

    void load();

    return () => {
      active = false;
    };
  }, [userId]);

  return state;
};
