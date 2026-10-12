"use client";

import { useState } from "react";
import type { FormEvent, ReactNode } from "react";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { ApiError } from "@/lib/api";

interface AuthFormProps {
  title: string;
  description: string;
  submitLabel: string;
  /** `current-password` for login, `new-password` for registration. */
  passwordAutoComplete: "current-password" | "new-password";
  /** Minimum password length enforced client-side; `0` disables the check. */
  passwordMinLength?: number;
  /** Performs the request; should throw `ApiError` on failure. */
  onSubmit: (email: string, password: string) => Promise<void>;
  footer: ReactNode;
}

interface CredentialsErrors {
  email?: string;
  password?: string;
}

const isValidEmail = (value: string): boolean => {
  const atIndex = value.indexOf("@");
  if (atIndex <= 0 || atIndex !== value.lastIndexOf("@")) {
    return false;
  }
  const domain = value.slice(atIndex + 1);
  return (
    domain.includes(".") &&
    !domain.startsWith(".") &&
    !domain.endsWith(".") &&
    !value.includes(" ")
  );
};

const validate = (
  email: string,
  password: string,
  passwordMinLength: number
): CredentialsErrors => {
  const errors: CredentialsErrors = {};
  const trimmedEmail = email.trim();

  if (!trimmedEmail) {
    errors.email = "Email is required";
  } else if (!isValidEmail(trimmedEmail)) {
    errors.email = "Enter a valid email address";
  }

  if (!password) {
    errors.password = "Password is required";
  } else if (passwordMinLength > 0 && password.length < passwordMinLength) {
    errors.password = `Password must be at least ${passwordMinLength} characters`;
  }

  return errors;
};

const serverFieldErrors = (
  fields: { field: string; message: string }[]
): CredentialsErrors => {
  const errors: CredentialsErrors = {};
  for (const field of fields) {
    if (field.field === "email") {
      errors.email = field.message;
    }
    if (field.field === "password") {
      errors.password = field.message;
    }
  }
  return errors;
};

/**
 * Shared email/password form used by the login and registration pages.
 *
 * Owns its input state, client-side validation, in-flight state (duplicate
 * submissions are prevented), and error presentation. The page supplies the
 * request itself via `onSubmit`, which is also responsible for navigating on
 * success.
 */
export const AuthForm = ({
  title,
  description,
  submitLabel,
  passwordAutoComplete,
  passwordMinLength = 0,
  onSubmit,
  footer,
}: AuthFormProps) => {
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [fieldErrors, setFieldErrors] = useState<CredentialsErrors>({});
  const [formError, setFormError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);

  const handleSubmit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (submitting) {
      return;
    }

    setFormError(null);

    const errors = validate(email, password, passwordMinLength);
    if (errors.email || errors.password) {
      setFieldErrors(errors);
      return;
    }
    setFieldErrors({});

    setSubmitting(true);
    try {
      await onSubmit(email, password);
      // On success the page navigates away; leave `submitting` set so the form
      // cannot be resubmitted during the transition.
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
      setSubmitting(false);
    }
  };

  return (
    <main className="bg-background text-foreground flex flex-1 items-center justify-center p-6">
      <div className="w-full max-w-sm">
        <div className="mb-6 flex flex-col gap-1.5 text-center">
          <h1 className="text-2xl font-semibold tracking-tight">{title}</h1>
          <p className="text-muted-foreground text-sm">{description}</p>
        </div>

        <form
          className="flex flex-col gap-4"
          noValidate
          onSubmit={handleSubmit}
        >
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="email">Email</Label>
            <Input
              aria-describedby={fieldErrors.email ? "email-error" : undefined}
              aria-invalid={fieldErrors.email ? true : undefined}
              autoComplete="email"
              disabled={submitting}
              id="email"
              name="email"
              onChange={(event) => setEmail(event.target.value)}
              required
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
            <Label htmlFor="password">Password</Label>
            <Input
              aria-describedby={
                fieldErrors.password ? "password-error" : undefined
              }
              aria-invalid={fieldErrors.password ? true : undefined}
              autoComplete={passwordAutoComplete}
              disabled={submitting}
              id="password"
              name="password"
              onChange={(event) => setPassword(event.target.value)}
              required
              type="password"
              value={password}
            />
            {fieldErrors.password ? (
              <p className="text-destructive text-sm" id="password-error">
                {fieldErrors.password}
              </p>
            ) : null}
          </div>

          {formError ? (
            <p className="text-destructive text-sm" role="alert">
              {formError}
            </p>
          ) : null}

          <Button className="w-full" disabled={submitting} type="submit">
            {submitting ? "Please wait…" : submitLabel}
          </Button>
        </form>

        <div className="text-muted-foreground mt-6 text-center text-sm">
          {footer}
        </div>
      </div>
    </main>
  );
};
