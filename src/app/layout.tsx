import type { Metadata } from "next";
import { Geist, Geist_Mono } from "next/font/google";
import "./globals.css";
import { SessionProvider } from "@/components/SessionProvider";
import { QueueProvider } from "@/components/QueueProvider";
import { PlayerProvider } from "@/components/PlayerProvider";
import { PlayChoiceProvider } from "@/components/PlayChoiceProvider";
import { WatchedProvider } from "@/components/WatchedProvider";
import { AppShell } from "@/components/AppShell";

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
      <body className="bg-background text-foreground">
        <SessionProvider>
          <WatchedProvider>
            <QueueProvider>
              <PlayerProvider>
                <PlayChoiceProvider>
                  <AppShell>{children}</AppShell>
                </PlayChoiceProvider>
              </PlayerProvider>
            </QueueProvider>
          </WatchedProvider>
        </SessionProvider>
      </body>
    </html>
  );
}
