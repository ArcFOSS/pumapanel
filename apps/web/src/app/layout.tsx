import type { Metadata } from "next";
import { Geist, Geist_Mono } from "next/font/google";

import { AuthProvider } from "@/components/auth-provider";

import "./globals.css";

const geistSans = Geist({
  subsets: ["latin"],
  variable: "--font-sans",
});

const geistMono = Geist_Mono({
  subsets: ["latin"],
  variable: "--font-mono",
});

export const metadata: Metadata = {
  description: "A production-minded Next.js + Rust monorepo.",
  title: "PumaPanel",
};

const RootLayout = ({ children }: LayoutProps<"/">) => (
  <html
    className={`${geistSans.variable} ${geistMono.variable} h-full antialiased`}
    lang="en"
  >
    <body className="flex min-h-full flex-col">
      <AuthProvider>{children}</AuthProvider>
    </body>
  </html>
);

export default RootLayout;
