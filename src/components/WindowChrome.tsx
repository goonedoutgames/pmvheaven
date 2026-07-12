"use client";

import { useCallback, useEffect, useState } from "react";
import { Copy, Minus, Square } from "lucide-react";
import { usePlayer } from "./PlayerProvider";

/** True when running inside the Tauri desktop shell (not a normal browser). */
export function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

type Dir =
  | "North"
  | "South"
  | "East"
  | "West"
  | "NorthEast"
  | "NorthWest"
  | "SouthEast"
  | "SouthWest";

async function win() {
  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  return getCurrentWindow();
}

/**
 * Custom desktop titlebar. The native window decorations are disabled
 * (`decorations: false`) because WebKitGTK draws over them on Linux, leaving
 * their buttons dead. This renders an app-styled titlebar with working
 * minimize / maximize / close controls, a drag region, and — since a
 * borderless window loses native resize — edge/corner resize handles.
 *
 * Renders nothing in a normal browser.
 */
export function WindowChrome() {
  const { fullscreen } = usePlayer();
  const [ready, setReady] = useState(false);
  const [maximized, setMaximized] = useState(false);

  useEffect(() => {
    if (!isTauri()) return;
    setReady(true);
    void (async () => {
      try {
        setMaximized(await (await win()).isMaximized());
      } catch {
        /* ignore */
      }
    })();
  }, []);

  // Collapse the titlebar (and its reserved space) while the player is fullscreen.
  useEffect(() => {
    if (!isTauri()) return;
    document.documentElement.style.setProperty("--titlebar-h", fullscreen ? "0px" : "36px");
  }, [fullscreen]);

  const minimize = useCallback(async () => {
    (await win()).minimize().catch(() => {});
  }, []);

  const toggleMaximize = useCallback(async () => {
    const w = await win();
    await w.toggleMaximize().catch(() => {});
    setMaximized(await w.isMaximized().catch(() => false));
  }, []);

  const close = useCallback(async () => {
    (await win()).close().catch(() => {});
  }, []);

  const startResize = useCallback(async (dir: Dir) => {
    try {
      // ResizeDirection is a string-literal union in this API version.
      await (await win()).startResizeDragging(dir);
    } catch {
      /* ignore */
    }
  }, []);

  if (!ready || fullscreen) return null;

  const edge = "fixed z-[300]";
  const handles: { dir: Dir; cls: string }[] = [
    { dir: "North", cls: "top-0 inset-x-2 h-1 cursor-ns-resize" },
    { dir: "South", cls: "bottom-0 inset-x-2 h-1 cursor-ns-resize" },
    { dir: "West", cls: "left-0 inset-y-2 w-1 cursor-ew-resize" },
    { dir: "East", cls: "right-0 inset-y-2 w-1 cursor-ew-resize" },
    { dir: "NorthWest", cls: "top-0 left-0 h-2 w-2 cursor-nwse-resize" },
    { dir: "NorthEast", cls: "top-0 right-0 h-2 w-2 cursor-nesw-resize" },
    { dir: "SouthWest", cls: "bottom-0 left-0 h-2 w-2 cursor-nesw-resize" },
    { dir: "SouthEast", cls: "bottom-0 right-0 h-2 w-2 cursor-nwse-resize" },
  ];

  return (
    <>
      {/* Titlebar */}
      <header
        data-tauri-drag-region
        onDoubleClick={toggleMaximize}
        className="fixed inset-x-0 top-0 z-[200] flex h-9 select-none items-center justify-between border-b border-border bg-background/90 pl-3 backdrop-blur-xl"
      >
        <div data-tauri-drag-region className="flex items-center gap-2">
          <span className="grid h-5 w-5 place-items-center rounded-md bg-gradient-to-br from-accent to-accent-2 text-[11px] font-black text-white">
            P
          </span>
          <span className="text-xs font-semibold tracking-tight text-muted">
            PMV<span className="text-accent">Heaven</span>
          </span>
        </div>

        <div className="flex h-full items-center">
          <button
            onClick={minimize}
            aria-label="Minimize"
            className="grid h-full w-11 place-items-center text-muted transition hover:bg-surface-2 hover:text-foreground"
          >
            <Minus size={15} />
          </button>
          <button
            onClick={toggleMaximize}
            aria-label={maximized ? "Restore" : "Maximize"}
            className="grid h-full w-11 place-items-center text-muted transition hover:bg-surface-2 hover:text-foreground"
          >
            {maximized ? <Copy size={13} /> : <Square size={13} />}
          </button>
          <button
            onClick={close}
            aria-label="Close"
            className="group/close grid h-full w-12 place-items-center text-muted transition hover:bg-rose-600"
          >
            {/* eslint-disable-next-line @next/next/no-img-element */}
            <img
              src="/sexy_close.svg"
              alt=""
              className="h-5 w-5 invert object-contain opacity-70 transition group-hover/close:opacity-100 stroke-2"
            />
          </button>
        </div>
      </header>

      {/* Resize handles (borderless window has no native resize) */}
      {handles.map((h) => (
        <div
          key={h.dir}
          onMouseDown={(e) => {
            e.preventDefault();
            void startResize(h.dir);
          }}
          className={`${edge} ${h.cls}`}
        />
      ))}
    </>
  );
}
