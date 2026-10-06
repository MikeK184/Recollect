import {
  Children,
  cloneElement,
  isValidElement,
  type CSSProperties,
  type ReactElement,
  type ReactNode,
} from "react";

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

/** Fade+rise entrance with a stagger for a list of children. Each child gets
 * the `rc-enter` class and an incremental animation delay; non-element nodes
 * are wrapped in a plain div so they animate too. */
export function Stagger({
  className,
  children,
}: {
  className?: string;
  children: ReactNode;
}) {
  const items = Children.toArray(children);
  return (
    <div className={className ? `rc-stagger ${className}` : "rc-stagger"}>
      {items.map((child, index) => {
        const delay = staggerStyle(index);
        if (!isValidElement(child))
          return (
            <div key={index} className="rc-enter" style={delay}>
              {child}
            </div>
          );
        const element = child as ReactElement<{
          className?: string;
          style?: CSSProperties;
        }>;
        const props = element.props;
        return cloneElement(element, {
          className: `rc-enter ${props.className ?? ""}`.trim(),
          style: { ...props.style, ...delay },
        });
      })}
    </div>
  );
}
