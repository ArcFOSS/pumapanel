"use client";

import Link from "next/link";
import { useRouter } from "next/navigation";

import { AuthForm } from "@/components/auth-form";
import { useAuth } from "@/components/auth-provider";

const LoginPage = () => {
  const router = useRouter();
  const { login } = useAuth();

  return (
    <AuthForm
      description="Sign in to your PumaPanel account."
      footer={
        <>
          Don&apos;t have an account?{" "}
          <Link
            className="text-foreground font-medium hover:underline"
            href="/register"
          >
            Create one
          </Link>
        </>
      }
      onSubmit={async (email, password) => {
        await login(email, password);
        router.push("/");
      }}
      passwordAutoComplete="current-password"
      submitLabel="Sign in"
      title="Welcome back"
    />
  );
};

export default LoginPage;
