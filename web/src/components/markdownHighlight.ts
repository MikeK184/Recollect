import { common, createLowlight } from "lowlight";

// This module and its grammars are fetched only when a bounded, labeled code
// block is displayed. Never guess a language or highlight unbounded inputs.
export const highlighter = createLowlight(common);
