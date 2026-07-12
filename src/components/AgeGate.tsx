"use client";

import { useEffect, useState } from "react";

const KEY = "ph_age_ok";

export function AgeGate() {
  const [ok, setOk] = useState(true);

  useEffect(() => {
    setOk(localStorage.getItem(KEY) === "1");
  }, []);

  if (ok) return null;

  return (
    <div className="fixed inset-0 z-[100] flex items-center justify-center bg-black/90 p-4 backdrop-blur">
      <div className="max-w-md rounded-2xl border border-border bg-surface p-8 text-center shadow-2xl">
        <h1 className="text-2xl font-bold">Adult content</h1>
        <p className="mt-3 text-sm text-muted">
          This site contains explicit material intended for adults only. By
          entering you confirm that you are at least 18 years old (or the age of
          majority in your jurisdiction) and consent to viewing adult content.
        </p>
        <div className="mt-6 flex flex-col gap-3">
          <button
            onClick={() => {
              localStorage.setItem(KEY, "1");
              setOk(true);
            }}
            className="rounded-lg bg-accent px-4 py-2.5 font-semibold text-white transition hover:opacity-90"
          >
            I am 18 or older — Enter
          </button>
          <a
            href="https://www.google.com"
            className="rounded-lg border border-border px-4 py-2.5 text-sm text-muted transition hover:bg-surface-2"
          >
            Leave
          </a>
        </div>
      </div>
    </div>
  );
}
