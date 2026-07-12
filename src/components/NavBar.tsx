"use client";

import Link from "next/link";
import { usePathname, useRouter } from "next/navigation";
import { useEffect, useRef, useState } from "react";
import {
  Clock,
  Flame,
  Heart,
  History,
  Home,
  LogIn,
  Search,
  Settings,
  Sparkles,
} from "lucide-react";
import { useSession } from "./SessionProvider";

const NAV = [
  { href: "/", label: "Home", icon: Home },
  { href: "/browse?sort=-bayesianRating", label: "Popular", icon: Flame },
  { href: "/browse?sort=-uploadDate", label: "Newest", icon: Sparkles },
];

const AUTH_NAV = [
  { href: "/history", label: "History", icon: History },
  { href: "/favorites", label: "Favorites", icon: Heart },
  { href: "/watch-later", label: "Watch Later", icon: Clock },
];

export function NavBar() {
  const { authenticated, user, loading } = useSession();
  const pathname = usePathname();
  const router = useRouter();
  const [q, setQ] = useState("");
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "/" && document.activeElement?.tagName !== "INPUT") {
        e.preventDefault();
        inputRef.current?.focus();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  const submitSearch = (e: React.FormEvent) => {
    e.preventDefault();
    if (q.trim()) router.push(`/search?q=${encodeURIComponent(q.trim())}`);
  };

  const isActive = (href: string) =>
    href === "/" ? pathname === "/" : pathname.startsWith(href.split("?")[0]);

  return (
    <header className="sticky top-0 z-50 border-b border-border bg-background/80 backdrop-blur-xl">
      <div className="mx-auto flex max-w-[1600px] items-center gap-3 px-3 py-3 sm:gap-5 sm:px-6">
        <Link href="/" className="flex shrink-0 items-center gap-2">
          <span className="grid h-8 w-8 place-items-center rounded-lg bg-gradient-to-br from-accent to-accent-2 font-black text-white">
            P
          </span>
          <span className="hidden text-lg font-bold tracking-tight sm:block">
            PMV<span className="text-accent">Heaven</span>
          </span>
        </Link>

        <nav className="hidden items-center gap-1 lg:flex">
          {[...NAV, ...(authenticated ? AUTH_NAV : [])].map(({ href, label, icon: Icon }) => (
            <Link
              key={href}
              href={href}
              className={`flex items-center gap-1.5 rounded-lg px-3 py-2 text-sm font-medium transition ${
                isActive(href)
                  ? "bg-surface-2 text-foreground"
                  : "text-muted hover:bg-surface hover:text-foreground"
              }`}
            >
              <Icon size={16} />
              {label}
            </Link>
          ))}
        </nav>

        <form onSubmit={submitSearch} className="relative ml-auto flex-1 max-w-md">
          <Search
            size={16}
            className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-muted"
          />
          <input
            ref={inputRef}
            value={q}
            onChange={(e) => setQ(e.target.value)}
            placeholder="Search videos, tags, creators…"
            className="w-full rounded-full border border-border bg-surface py-2 pl-9 pr-4 text-sm outline-none transition focus:border-accent/60"
          />
        </form>

        {loading ? (
          <div className="h-8 w-8 animate-pulse rounded-full bg-surface-2" />
        ) : authenticated ? (
          <div className="flex items-center gap-2">
            <Link
              href="/settings"
              className="hidden rounded-lg p-2 text-muted transition hover:bg-surface hover:text-foreground sm:block"
              title="Settings"
            >
              <Settings size={18} />
            </Link>
            <Link href="/settings" className="flex items-center gap-2" title={user?.username}>
              {user?.avatarUrl ? (
                // eslint-disable-next-line @next/next/no-img-element
                <img
                  src={user.avatarUrl}
                  alt={user.username}
                  className="h-8 w-8 rounded-full object-cover ring-1 ring-border"
                />
              ) : (
                <span className="grid h-8 w-8 place-items-center rounded-full bg-surface-2 text-sm font-bold">
                  {user?.username?.[0]?.toUpperCase() ?? "?"}
                </span>
              )}
            </Link>
          </div>
        ) : (
          <Link
            href="/login"
            className="flex shrink-0 items-center gap-1.5 rounded-lg bg-accent px-3.5 py-2 text-sm font-semibold text-white transition hover:opacity-90"
          >
            <LogIn size={16} />
            <span className="hidden sm:inline">Sign in</span>
          </Link>
        )}
      </div>
    </header>
  );
}
