"use client";

import { usePathname } from "next/navigation";
import { NavBar } from "./NavBar";
import { AgeGate } from "./AgeGate";
import { WindowChrome } from "./WindowChrome";
import { QueuePanel } from "./QueuePanel";
import { PlayerRail } from "./PlayerRail";
import { usePlayer } from "./PlayerProvider";

/**
 * Top-level layout switch. The dedicated player window (route `/player`) renders
 * bare — just the player, no app chrome. Every other route gets the full app: a
 * scrolling content column that reflows to make room for the docked player rail.
 */
export function AppShell({ children }: { children: React.ReactNode }) {
  const pathname = usePathname();
  const { fullscreen } = usePlayer();

  if (pathname === "/player") {
    return <div className="h-full min-h-0 flex-1">{children}</div>;
  }

  return (
    <>
      <WindowChrome />
      <AgeGate />
      <div className="flex min-h-0 flex-1 flex-col lg:flex-row">
        {/* Hidden (not unmounted) while fullscreen so the rail can fill the
            window and the player element keeps playing without a restart. */}
        <div
          className={`app-scroll order-2 min-w-0 flex-1 flex-col lg:order-1 ${
            fullscreen ? "hidden" : "flex"
          }`}
        >
          <NavBar />
          <main className="mx-auto w-full max-w-[1600px] flex-1 px-3 py-6 sm:px-6">
            {children}
          </main>
        </div>
        <PlayerRail />
      </div>
      <QueuePanel />
    </>
  );
}
