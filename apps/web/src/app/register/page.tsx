"use client";

import Link from "next/link";
import { useRouter } from "next/navigation";

import { AuthForm } from "@/components/auth-form";
import { useAuth } from "@/components/auth-provider";

const MIN_PASSWORD_LENGTH = 8;

const RegisterPage = () => {
  const router = useRouter();
  const { register } = useAuth();

  return (
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
      onSubmit={async (email, password) => {
        await register(email, password);
        router.push("/");
      }}
      passwordAutoComplete="new-password"
      passwordMinLength={MIN_PASSWORD_LENGTH}
      submitLabel="Create account"
      title="Create your account"
    />
  );
};

export default RegisterPage;
