import type { CSSProperties } from "react";

// Entrance stagger step in ms; mirrors --rc-stagger-step in the token layer.
const STAGGER_STEP = 40;
// Cap the total stagger so long lists settle quickly (12 steps = 480ms).
const MAX_STAGGER_STEPS = 12;

/** Inline delay for one item of a staggered entrance. Pair with the
 * `rc-enter` class, which owns the fade+rise animation itself. */
export function staggerStyle(index: number): CSSProperties {
  return {
    animationDelay: `${Math.min(index, MAX_STAGGER_STEPS) * STAGGER_STEP}ms`,
  };
}
