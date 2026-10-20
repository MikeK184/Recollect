import type { Transition, Variants } from "motion/react";

// Motion presets from the design system (motion.md). Animate transform,
// opacity and filter only; layout animations handle size and position.
export const snap: Transition = {
  type: "spring",
  visualDuration: 0.16,
  bounce: 0.2,
};
export const glide: Transition = {
  type: "spring",
  visualDuration: 0.22,
  bounce: 0,
};
export const settle: Transition = {
  type: "spring",
  visualDuration: 0.36,
  bounce: 0,
};
export const arrive: Transition = { duration: 0.22, ease: [0.16, 1, 0.3, 1] };
export const leave: Transition = { duration: 0.16, ease: [0.4, 0, 1, 1] };

/** Seconds between list items entering; at most ten steps. */
export const staggerStep = 0.03;
export const maxStaggerSteps = 10;
export const staggerDelay = (index: number) =>
  Math.min(index, maxStaggerSteps) * staggerStep;

/** Page content fades and rises 4px on route change. */
export const pageEnter: Variants = {
  initial: { opacity: 0, y: 4 },
  animate: { opacity: 1, y: 0, transition: arrive },
};

/** A list item entering on first load only (never on refetch). */
export const listItem: Variants = {
  initial: { opacity: 0, y: 4 },
  animate: (index: number = 0) => ({
    opacity: 1,
    y: 0,
    transition: { ...arrive, delay: staggerDelay(index) },
  }),
};

/** Overlays: menus pop from 0.96, dialogs rise 8px from 0.98. */
export const popIn: Variants = {
  initial: { opacity: 0, scale: 0.96 },
  animate: { opacity: 1, scale: 1, transition: arrive },
  exit: { opacity: 0, scale: 0.96, transition: leave },
};
export const dialogIn: Variants = {
  initial: { opacity: 0, y: 8, scale: 0.98 },
  animate: { opacity: 1, y: 0, scale: 1, transition: arrive },
  exit: { opacity: 0, y: 4, scale: 0.98, transition: leave },
};
/** The inspector slides 16px in from the right. */
export const paneIn: Variants = {
  initial: { opacity: 0, x: 16 },
  animate: { opacity: 1, x: 0, transition: settle },
  exit: { opacity: 0, x: 16, transition: leave },
};
/** A validated answer resolves from 4px blur; it never streams. */
export const reveal: Variants = {
  initial: { opacity: 0, filter: "blur(4px)" },
  animate: {
    opacity: 1,
    filter: "blur(0px)",
    transition: { duration: 0.36, ease: [0.16, 1, 0.3, 1] },
  },
};
