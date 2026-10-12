"use client";

import { Eye, EyeOff } from "lucide-react";
import { useState } from "react";
import type { FormEvent, ReactNode } from "react";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { ApiError } from "@/lib/api";
import type { FieldError } from "@/lib/api";
import { validateEmail, validateUsername } from "@/lib/validate";

interface AuthFormBaseProps {
  description: string;
  footer: ReactNode;
  submitLabel: string;
  title: string;
}

interface LoginFormProps extends AuthFormBaseProps {
  variant: "login";
  /** `identifier` is an email address or a username. */
  onSubmit: (identifier: string, password: string) => Promise<void>;
}

interface RegisterFormProps extends AuthFormBaseProps {
  variant: "register";
  /** Minimum password length enforced client-side; `0` disables the check. */
  passwordMinLength?: number;
  onSubmit: (
    username: string,
    email: string,
    password: string
  ) => Promise<void>;
}

type AuthFormProps = LoginFormProps | RegisterFormProps;

interface AuthFormErrors {
  email?: string;
  identifier?: string;
  password?: string;
  username?: string;
}

type ErrorField = keyof AuthFormErrors;

const passwordError = (
  value: string,
  minLength: number
): string | undefined => {
  if (!value) {
    return "Password is required";
  }
  if (minLength > 0 && value.length < minLength) {
    return `Password must be at least ${minLength} characters`;
  }
  return undefined;
};

const addError = (
  errors: AuthFormErrors,
  field: ErrorField,
  message: string | undefined
): void => {
  if (message) {
    errors[field] = message;
  }
};

const validateLogin = (
  identifier: string,
  password: string
): AuthFormErrors => {
  const errors: AuthFormErrors = {};
  if (!identifier.trim()) {
    errors.identifier = "Email or username is required";
  }
  addError(errors, "password", passwordError(password, 0));
  return errors;
};

const validateRegister = (
  username: string,
  email: string,
  password: string,
  passwordMinLength: number
): AuthFormErrors => {
  const errors: AuthFormErrors = {};
  addError(errors, "username", validateUsername(username));
  addError(errors, "email", validateEmail(email));
  addError(errors, "password", passwordError(password, passwordMinLength));
  return errors;
};

const hasErrors = (errors: AuthFormErrors): boolean =>
  Object.keys(errors).length > 0;

const serverFieldErrors = (fields: FieldError[]): AuthFormErrors => {
  const errors: AuthFormErrors = {};
  for (const { field, message } of fields) {
    switch (field) {
      case "email":
      case "identifier":
      case "password":
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

interface FormFieldProps {
  autoComplete: string;
  disabled: boolean;
  error?: string;
  id: string;
  label: string;
  onChange: (value: string) => void;
  type?: string;
  value: string;
}

const FormField = ({
  autoComplete,
  disabled,
  error,
  id,
  label,
  onChange,
  type = "text",
  value,
}: FormFieldProps) => {
  const [revealed, setRevealed] = useState(false);
  const isPassword = type === "password";

  return (
    <div className="flex flex-col gap-1.5">
      <Label htmlFor={id}>{label}</Label>
      <div className="flex items-center gap-2">
        <Input
          aria-describedby={error ? `${id}-error` : undefined}
          aria-invalid={error ? true : undefined}
          autoComplete={autoComplete}
          disabled={disabled}
          id={id}
          name={id}
          onChange={(event) => onChange(event.target.value)}
          required
          type={isPassword && revealed ? "text" : type}
          value={value}
        />
        {isPassword ? (
          <Button
            aria-label={revealed ? "Hide password" : "Show password"}
            disabled={disabled}
            onClick={() => setRevealed((current) => !current)}
            size="icon"
            type="button"
            variant="ghost"
          >
            {revealed ? <EyeOff /> : <Eye />}
          </Button>
        ) : null}
      </div>
      {error ? (
        <p className="text-destructive text-sm" id={`${id}-error`}>
          {error}
        </p>
      ) : null}
    </div>
  );
};

/**
 * Shared login/registration form.
 *
 * Login collects an email-or-username identifier; registration additionally
 * collects a required username. The form owns its input state, client-side
 * validation, in-flight state (duplicate submissions are prevented), and error
 * presentation. The page supplies the request itself via `onSubmit`, which is
 * also responsible for navigating on success.
 */
export const AuthForm = (props: AuthFormProps) => {
  const { description, footer, submitLabel, title } = props;

  const [identifier, setIdentifier] = useState("");
  const [username, setUsername] = useState("");
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [fieldErrors, setFieldErrors] = useState<AuthFormErrors>({});
  const [formError, setFormError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);

  const submitCredentials = async (run: () => Promise<void>) => {
    setFieldErrors({});
    setSubmitting(true);
    try {
      // On success the page navigates away; leave `submitting` set so the form
      // cannot be resubmitted during the transition.
      await run();
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

  const handleSubmit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (submitting) {
      return;
    }
    setFormError(null);

    if (props.variant === "register") {
      const errors = validateRegister(
        username,
        email,
        password,
        props.passwordMinLength ?? 0
      );
      if (hasErrors(errors)) {
        setFieldErrors(errors);
        return;
      }
      await submitCredentials(() => props.onSubmit(username, email, password));
      return;
    }

    const errors = validateLogin(identifier, password);
    if (hasErrors(errors)) {
      setFieldErrors(errors);
      return;
    }
    await submitCredentials(() => props.onSubmit(identifier, password));
  };

  return (
    <div className="w-full max-w-sm">
      <div className="border-border bg-card rounded-2xl border p-6 shadow-sm sm:p-8">
        <div className="mb-6 flex flex-col gap-1.5">
          <h1 className="text-2xl font-semibold tracking-tight">{title}</h1>
          <p className="text-muted-foreground text-sm">{description}</p>
        </div>

        <form
          className="flex flex-col gap-4"
          noValidate
          onSubmit={handleSubmit}
        >
          {props.variant === "register" ? (
            <>
              <FormField
                autoComplete="username"
                disabled={submitting}
                error={fieldErrors.username}
                id="username"
                label="Username"
                onChange={setUsername}
                value={username}
              />
              <FormField
                autoComplete="email"
                disabled={submitting}
                error={fieldErrors.email}
                id="email"
                label="Email"
                onChange={setEmail}
                type="email"
                value={email}
              />
            </>
          ) : (
            <FormField
              autoComplete="username"
              disabled={submitting}
              error={fieldErrors.identifier}
              id="identifier"
              label="Email or username"
              onChange={setIdentifier}
              value={identifier}
            />
          )}

          <FormField
            autoComplete={
              props.variant === "register" ? "new-password" : "current-password"
            }
            disabled={submitting}
            error={fieldErrors.password}
            id="password"
            label="Password"
            onChange={setPassword}
            type="password"
            value={password}
          />

          {formError ? (
            <p className="text-destructive text-sm" role="alert">
              {formError}
            </p>
          ) : null}

          <Button className="w-full" disabled={submitting} type="submit">
            {submitting ? "Please wait…" : submitLabel}
          </Button>
        </form>
      </div>

      <div className="text-muted-foreground mt-6 text-center text-sm">
        {footer}
      </div>
    </div>
  );
};
