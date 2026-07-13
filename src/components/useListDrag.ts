"use client";

import { useCallback, useRef, useState } from "react";

/**
 * Pointer-based drag reordering for a vertical list.
 *
 * We deliberately avoid the HTML5 drag-and-drop API: WebKitGTK (the Tauri Linux
 * webview) implements it unreliably — drags never register as valid drops and
 * you just get a "no-drop" cursor. Plain pointer events work everywhere.
 *
 * Mark each list item with `data-drag-index={i}` and start a drag from a handle
 * via `onPointerDown={(e) => start(i, e)}`. `dragIndex` is the item being moved;
 * `overIndex` is the current drop target — use them for visual feedback. The
 * actual move fires via `onReorder(from, to)` on release.
 */
export function useListDrag(onReorder: (from: number, to: number) => void) {
  const [dragIndex, setDragIndex] = useState<number | null>(null);
  const [overIndex, setOverIndex] = useState<number | null>(null);
  const fromRef = useRef<number | null>(null);
  const toRef = useRef<number | null>(null);

  const start = useCallback(
    (index: number, e: React.PointerEvent) => {
      // Left button / touch / pen only; ignore right-click etc.
      if (e.button !== 0 && e.pointerType === "mouse") return;
      e.preventDefault();
      fromRef.current = index;
      toRef.current = index;
      setDragIndex(index);
      setOverIndex(index);

      const onMove = (ev: PointerEvent) => {
        const el = document.elementFromPoint(ev.clientX, ev.clientY);
        const li = el?.closest<HTMLElement>("[data-drag-index]");
        if (!li) return;
        const idx = Number(li.dataset.dragIndex);
        if (!Number.isNaN(idx) && idx !== toRef.current) {
          toRef.current = idx;
          setOverIndex(idx);
        }
      };
      const onUp = () => {
        const from = fromRef.current;
        const to = toRef.current;
        if (from !== null && to !== null && from !== to) onReorder(from, to);
        fromRef.current = null;
        toRef.current = null;
        setDragIndex(null);
        setOverIndex(null);
        window.removeEventListener("pointermove", onMove);
        window.removeEventListener("pointerup", onUp);
        window.removeEventListener("pointercancel", onUp);
      };

      window.addEventListener("pointermove", onMove);
      window.addEventListener("pointerup", onUp);
      window.addEventListener("pointercancel", onUp);
    },
    [onReorder],
  );

  return { dragIndex, overIndex, start };
}
