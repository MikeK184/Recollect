import type { CSSProperties } from "react";
import { maxStaggerSteps } from "../design/motion";

// Entrance stagger step in ms (30ms, at most ten steps) for CSS entrances.
// Pair with the `animate-enter` utility; Motion lists use design/motion.ts.
const STAGGER_STEP = 30;

/** Inline delay for one item of a staggered CSS entrance (first load only). */
export function staggerStyle(index: number): CSSProperties {
  return {
    animationDelay: `${Math.min(index, maxStaggerSteps) * STAGGER_STEP}ms`,
  };
}
