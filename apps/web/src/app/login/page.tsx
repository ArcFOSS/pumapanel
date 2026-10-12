"use client";

import Link from "next/link";
import { useRouter } from "next/navigation";

import { AuthForm } from "@/components/auth-form";
import { useAuth } from "@/components/auth-provider";
import { AuthShell } from "@/components/auth-shell";

const LoginPage = () => {
  const router = useRouter();
  const { login } = useAuth();

  return (
    <AuthShell>
      <AuthForm
        description="Sign in with your email or username."
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
        onSubmit={async (identifier, password) => {
          await login(identifier, password);
          router.push("/dashboard");
        }}
        submitLabel="Sign in"
        title="Welcome back"
        variant="login"
      />
    </AuthShell>
  );
};

export default LoginPage;
