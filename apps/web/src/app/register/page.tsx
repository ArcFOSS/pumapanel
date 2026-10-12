"use client";

import Link from "next/link";
import { useRouter } from "next/navigation";

import { AuthForm } from "@/components/auth-form";
import { useAuth } from "@/components/auth-provider";
import { AuthShell } from "@/components/auth-shell";

const MIN_PASSWORD_LENGTH = 8;

const RegisterPage = () => {
  const router = useRouter();
  const { register } = useAuth();

  return (
    <AuthShell>
      <AuthForm
        description="Create an account to get started."
        footer={
          <>
            Already have an account?{" "}
            <Link
              className="text-foreground font-medium hover:underline"
              href="/login"
            >
              Sign in
            </Link>
          </>
        }
        onSubmit={async (username, email, password) => {
          await register(username, email, password);
          router.push("/dashboard");
        }}
        passwordMinLength={MIN_PASSWORD_LENGTH}
        submitLabel="Create account"
        title="Create your account"
        variant="register"
      />
    </AuthShell>
  );
};

export default RegisterPage;
