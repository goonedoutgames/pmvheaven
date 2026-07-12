import type { Metadata } from "next";
import { Geist, Geist_Mono } from "next/font/google";
import "./globals.css";
import { SessionProvider } from "@/components/SessionProvider";
import { NavBar } from "@/components/NavBar";
import { AgeGate } from "@/components/AgeGate";

const geistSans = Geist({
  variable: "--font-geist-sans",
  subsets: ["latin"],
});

const geistMono = Geist_Mono({
  variable: "--font-geist-mono",
  subsets: ["latin"],
});

export const metadata: Metadata = {
  title: "PMVHeaven",
  description: "A sleek, ad-free frontend for PMVHaven with permanent watch history.",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html
      lang="en"
      className={`${geistSans.variable} ${geistMono.variable} h-full antialiased`}
    >
      <body className="min-h-full flex flex-col bg-background text-foreground">
        <SessionProvider>
          <AgeGate />
          <NavBar />
          <main className="flex-1 w-full mx-auto max-w-[1600px] px-3 sm:px-6 py-6">
            {children}
          </main>
        </SessionProvider>
      </body>
    </html>
  );
}
