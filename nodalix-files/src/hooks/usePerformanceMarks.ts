import { useCallback, useEffect, useRef } from "react";

const marks = new Map<string, number>();
let debug = false;

export function setPerfDebug(enabled: boolean) {
  debug = enabled;
}

export function perfMark(label: string) {
  const now = performance.now();
  marks.set(label, now);
  if (debug) {
    const start = marks.get("app_start") ?? now;
    console.info(`[nodalix-files +${(now - start).toFixed(1)}ms] ${label}`);
  }
}

export function usePerformanceMarks(enabled: boolean) {
  const once = useRef(false);

  useEffect(() => {
    setPerfDebug(enabled);
    if (!once.current) {
      perfMark("app_start");
      once.current = true;
    }
  }, [enabled]);

  const mark = useCallback((label: string) => {
    perfMark(label);
  }, []);

  return { mark };
}
