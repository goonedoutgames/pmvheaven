"use client";

import { useRouter } from "next/navigation";
import { useState } from "react";
import { Loader2, LogIn, ShieldCheck } from "lucide-react";
import { useSession } from "@/components/SessionProvider";

export default function LoginPage() {
  const router = useRouter();
  const { refresh } = useSession();
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  const submit = async (e: React.FormEvent) => {
    e.preventDefault();
    setLoading(true);
    setError(null);
    try {
      const res = await fetch("/api/auth/login", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ email, password }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data.error ?? "Sign in failed");
      await refresh();
      router.push("/history");
    } catch (err) {
      setError(err instanceof Error ? err.message : "Sign in failed");
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="mx-auto flex max-w-md flex-col gap-6 py-10">
      <div className="text-center">
        <h1 className="text-2xl font-bold">Connect your PMVHaven account</h1>
        <p className="mt-2 text-sm text-muted">
          Sign in with your PMVHaven credentials to unlock permanent watch
          history, favorites, and personalized search.
        </p>
      </div>

      <form
        onSubmit={submit}
        className="flex flex-col gap-4 rounded-2xl border border-border bg-surface p-6"
      >
        <label className="flex flex-col gap-1.5 text-sm">
          <span className="font-medium">Email</span>
          <input
            type="email"
            required
            value={email}
            onChange={(e) => setEmail(e.target.value)}
            autoComplete="username"
            className="rounded-lg border border-border bg-background px-3 py-2.5 outline-none transition focus:border-accent/60"
          />
        </label>
        <label className="flex flex-col gap-1.5 text-sm">
          <span className="font-medium">Password</span>
          <input
            type="password"
            required
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            autoComplete="current-password"
            className="rounded-lg border border-border bg-background px-3 py-2.5 outline-none transition focus:border-accent/60"
          />
        </label>

        {error && (
          <p className="rounded-lg bg-rose-500/10 px-3 py-2 text-sm text-rose-400">
            {error}
          </p>
        )}

        <button
          type="submit"
          disabled={loading}
          className="flex items-center justify-center gap-2 rounded-lg bg-accent px-4 py-2.5 font-semibold text-white transition hover:opacity-90 disabled:opacity-60"
        >
          {loading ? <Loader2 size={18} className="animate-spin" /> : <LogIn size={18} />}
          Sign in
        </button>
      </form>

      <p className="flex items-start gap-2 rounded-xl border border-border bg-surface/50 p-4 text-xs text-muted">
        <ShieldCheck size={16} className="mt-0.5 shrink-0" />
        Your credentials are sent only to PMVHaven to obtain a session, then
        encrypted and stored locally on this device. They are never exposed to
        the browser or any third party.
      </p>
    </div>
  );
}
