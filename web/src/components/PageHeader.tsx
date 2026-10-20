import type { ReactNode } from "react";
import { motion } from "motion/react";
import { cn } from "../lib/utils";
import { pageEnter } from "../design/motion";

/**
 * Compact sans page header (ADR 0025): 28/34 title, one-line description, an
 * optional row of badges above the title and at most two actions. No page uses
 * an eyebrow; only Ask uses the serif hero instead of this header.
 */
export function PageHeader({
  title,
  description,
  meta,
  actions,
  className,
}: {
  title: ReactNode;
  description?: ReactNode;
  /** Small badges (scope, role) shown above the title. */
  meta?: ReactNode;
  /** @deprecated Eyebrows are retired; the value is ignored. */
  eyebrow?: string;
  actions?: ReactNode;
  className?: string;
}) {
  return (
    <motion.div
      variants={pageEnter}
      initial="initial"
      animate="animate"
      className={cn("flex items-end justify-between gap-6", className)}
    >
      <div className="min-w-0">
        {meta && <div className="mb-2.5 flex flex-wrap gap-1.5">{meta}</div>}
        <h1 className="m-0 text-balance text-page font-semibold text-foreground">
          {title}
        </h1>
        {description && (
          <p className="m-0 mt-1 max-w-[68ch] text-body text-muted-foreground">
            {description}
          </p>
        )}
      </div>
      {actions && <div className="flex shrink-0 items-center gap-2">{actions}</div>}
    </motion.div>
  );
}

/** Section heading inside a page: 20/28 with optional trailing actions. */
export function SectionHeader({
  title,
  description,
  actions,
  as: Heading = "h2",
  className,
}: {
  title: ReactNode;
  description?: ReactNode;
  actions?: ReactNode;
  as?: "h2" | "h3";
  className?: string;
}) {
  return (
    <div className={cn("flex items-end justify-between gap-4", className)}>
      <div className="min-w-0">
        <Heading
          className={cn(
            "m-0 font-semibold text-foreground",
            Heading === "h2" ? "text-section" : "text-title",
          )}
        >
          {title}
        </Heading>
        {description && (
          <p className="m-0 mt-0.5 text-body text-muted-foreground">{description}</p>
        )}
      </div>
      {actions && <div className="flex shrink-0 items-center gap-2">{actions}</div>}
    </div>
  );
}
