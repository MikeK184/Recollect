import { clsx, type ClassValue } from "clsx";
import { extendTailwindMerge } from "tailwind-merge";

// tailwind-merge must know the custom text scale, or `text-body text-muted-
// foreground` would be treated as two conflicting text utilities.
const merge = extendTailwindMerge({
  extend: {
    classGroups: {
      "font-size": [
        {
          text: [
            "caption",
            "label",
            "body",
            "title",
            "answer",
            "section",
            "page",
            "hero",
          ],
        },
      ],
      shadow: [{ shadow: ["focus", "glow"] }],
    },
  },
});

/** Join class names, letting later Tailwind utilities win over earlier ones. */
export function cn(...inputs: ClassValue[]) {
  return merge(clsx(inputs));
}
