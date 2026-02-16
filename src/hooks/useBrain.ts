import { useEffect, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";

/** Hook for brain event listeners */
export function useBrainEvents(callbacks?: {
  onMemoryStored?: (id: string) => void;
  onThoughtGenerated?: (thought: unknown) => void;
  onCycleCompleted?: () => void;
}) {
  useEffect(() => {
    const unlisteners: (() => void)[] = [];

    if (callbacks?.onMemoryStored) {
      listen<string>("memory_stored", (event) => {
        callbacks.onMemoryStored?.(event.payload);
      }).then((fn) => unlisteners.push(fn));
    }

    if (callbacks?.onThoughtGenerated) {
      listen("thought_generated", (event) => {
        callbacks.onThoughtGenerated?.(event.payload);
      }).then((fn) => unlisteners.push(fn));
    }

    if (callbacks?.onCycleCompleted) {
      listen("cycle_completed", () => {
        callbacks.onCycleCompleted?.();
      }).then((fn) => unlisteners.push(fn));
    }

    return () => {
      unlisteners.forEach((fn) => fn());
    };
  }, [callbacks]);
}

/** Hook for triggering brain actions */
export function useBrainActions() {
  const evolve = useCallback(async () => {
    return invoke("evolve");
  }, []);

  const cycle = useCallback(async () => {
    return invoke("cycle");
  }, []);

  const flush = useCallback(async () => {
    return invoke("flush");
  }, []);

  return { evolve, cycle, flush };
}
