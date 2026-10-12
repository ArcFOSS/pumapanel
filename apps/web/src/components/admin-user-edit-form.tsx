"use client";

import { ArrowLeft, LayoutDashboard } from "lucide-react";
import Link from "next/link";
import { useState } from "react";
import type { FormEvent } from "react";

import { Button, buttonVariants } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { ApiError, adminApi } from "@/lib/api";
import type { FieldError, UserRole } from "@/lib/api";
import { cn } from "@/lib/utils";
import { validateEmail, validateUsername } from "@/lib/validate";

interface AdminUserEditFormProps {
  /** Whether the account being edited is the signed-in administrator. */
  isSelf: boolean;
  user: {
    email: string;
    id: string;
    role: UserRole;
    username: string;
  };
}

interface EditFormErrors {
  email?: string;
  role?: string;
  username?: string;
}

/**
 * Draft edits layered over the stored profile.
 *
 * The form never copies the profile into state — the displayed value is
 * `draft.field ?? user.field` — so navigating between users cannot show stale
 * data, and only edited fields are held locally.
 */
interface EditDraft {
  email?: string;
  role?: UserRole;
  username?: string;
}

const addError = (
  errors: EditFormErrors,
  field: keyof EditFormErrors,
  message: string | undefined
): void => {
  if (message) {
    errors[field] = message;
  }
};

const serverFieldErrors = (fields: FieldError[]): EditFormErrors => {
  const errors: EditFormErrors = {};
  for (const { field, message } of fields) {
    switch (field) {
      case "email":
      case "role":
      case "username": {
        errors[field] = message;
        break;
      }
      default: {
        break;
      }
    }
  }
  return errors;
};

const hasErrors = (errors: EditFormErrors): boolean =>
  Object.keys(errors).length > 0;

/**
 * Admin form for editing a user's permitted details (username, email, role).
 *
 * Mirrors the auth form's conventions: client-side validation with the same
 * rules as the API, in-flight state, and field/generic error presentation.
 * Submitting a role change for your own account is impossible in the UI (the
 * selector is disabled), and the API rejects it as a second line of defense.
 */
export const AdminUserEditForm = ({ isSelf, user }: AdminUserEditFormProps) => {
  const [draft, setDraft] = useState<EditDraft>({});
  const [fieldErrors, setFieldErrors] = useState<EditFormErrors>({});
  const [formError, setFormError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);
  const [saved, setSaved] = useState(false);

  const username = draft.username ?? user.username;
  const email = draft.email ?? user.email;
  const role = draft.role ?? user.role;

  const setUsername = (value: string) => {
    setDraft((previous) => ({ ...previous, username: value }));
  };

  const setEmail = (value: string) => {
    setDraft((previous) => ({ ...previous, email: value }));
  };

  const setRole = (value: UserRole) => {
    setDraft((previous) => ({ ...previous, role: value }));
  };

  const handleSubmit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (submitting) {
      return;
    }
    setFormError(null);
    setSaved(false);

    const errors: EditFormErrors = {};
    addError(errors, "username", validateUsername(username));
    addError(errors, "email", validateEmail(email));
    if (hasErrors(errors)) {
      setFieldErrors(errors);
      return;
    }

    setFieldErrors({});
    setSubmitting(true);
    try {
      await adminApi.updateUser(user.id, { email, role, username });
      setSaved(true);
    } catch (error) {
      if (error instanceof ApiError) {
        const fields = error.fields ?? [];
        if (fields.length > 0) {
          setFieldErrors(serverFieldErrors(fields));
        } else {
          setFormError(error.message);
        }
      } else {
        setFormError("Something went wrong. Please try again.");
      }
    }
    setSubmitting(false);
  };

  return (
    <div className="mx-auto flex w-full max-w-3xl flex-col gap-6 px-6 py-10">
      <div className="flex items-center justify-between gap-4">
        <Link
          className={cn(
            buttonVariants({ size: "sm", variant: "ghost" }),
            "text-muted-foreground"
          )}
          href={`/admin/u/${user.id}`}
        >
          <ArrowLeft className="size-4" />
          Back to profile
        </Link>
        <Link
          className={cn(buttonVariants({ size: "sm", variant: "outline" }))}
          href="/dashboard"
        >
          <LayoutDashboard className="size-4" />
          Dashboard
        </Link>
      </div>

      <div className="border-border bg-card rounded-2xl border p-6 shadow-sm sm:p-8">
        <div className="mb-6 flex flex-col gap-1.5">
          <h1 className="text-2xl font-semibold tracking-tight">
            Edit {user.username}
          </h1>
          <p className="text-muted-foreground text-sm">
            Update the account&apos;s username, email, or role.
          </p>
        </div>

        <form
          className="flex flex-col gap-4"
          noValidate
          onSubmit={handleSubmit}
        >
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="username">Username</Label>
            <Input
              aria-describedby={
                fieldErrors.username ? "username-error" : undefined
              }
              aria-invalid={fieldErrors.username ? true : undefined}
              autoComplete="off"
              disabled={submitting}
              id="username"
              name="username"
              onChange={(event) => setUsername(event.target.value)}
              value={username}
            />
            {fieldErrors.username ? (
              <p className="text-destructive text-sm" id="username-error">
                {fieldErrors.username}
              </p>
            ) : null}
          </div>

          <div className="flex flex-col gap-1.5">
            <Label htmlFor="email">Email</Label>
            <Input
              aria-describedby={fieldErrors.email ? "email-error" : undefined}
              aria-invalid={fieldErrors.email ? true : undefined}
              autoComplete="off"
              disabled={submitting}
              id="email"
              name="email"
              onChange={(event) => setEmail(event.target.value)}
              type="email"
              value={email}
            />
            {fieldErrors.email ? (
              <p className="text-destructive text-sm" id="email-error">
                {fieldErrors.email}
              </p>
            ) : null}
          </div>

          <div className="flex flex-col gap-1.5">
            <Label htmlFor="role">Role</Label>
            <select
              aria-describedby={isSelf ? "role-note" : undefined}
              aria-invalid={fieldErrors.role ? true : undefined}
              className="border-input focus-visible:ring-ring/50 disabled:bg-input/50 focus-visible:border-ring h-8 w-full rounded-lg border bg-transparent px-2.5 text-base outline-none focus-visible:ring-3 disabled:cursor-not-allowed disabled:opacity-50 md:text-sm"
              disabled={submitting || isSelf}
              id="role"
              name="role"
              onChange={(event) =>
                setRole(event.target.value === "admin" ? "admin" : "user")
              }
              value={role}
            >
              <option value="user">User</option>
              <option value="admin">Admin</option>
            </select>
            {isSelf ? (
              <p className="text-muted-foreground text-sm" id="role-note">
                You cannot change your own role.
              </p>
            ) : null}
            {fieldErrors.role ? (
              <p className="text-destructive text-sm" id="role-error">
                {fieldErrors.role}
              </p>
            ) : null}
          </div>

          {formError ? (
            <p className="text-destructive text-sm" role="alert">
              {formError}
            </p>
          ) : null}

          {saved ? (
            <output className="text-brand-strong block text-sm">
              User updated.
            </output>
          ) : null}

          <Button disabled={submitting} type="submit">
            {submitting ? "Saving…" : "Save changes"}
          </Button>
        </form>
      </div>
    </div>
  );
};
