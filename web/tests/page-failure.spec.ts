import { test, expect } from "@playwright/test";
import { pageFailure } from "../src/components/pageFailureCopy";

test("lazy page failures give safe recovery copy across browser messages", () => {
  for (const message of [
    "Failed to fetch dynamically imported module: http://127.0.0.1/assets/old.js",
    "Importing a module script failed.",
    "error loading dynamically imported module: /assets/old.js",
    "Loading chunk example failed.",
  ]) {
    const copy = pageFailure(new Error(message));
    expect(copy.title).toBe("This page needs an update");
    expect(JSON.stringify(copy)).not.toContain("old.js");
    expect(copy.description).toContain("check your connection");
  }
  expect(pageFailure(new Error("Internal private detail")).title).toBe(
    "This page couldn't open",
  );
  expect(
    JSON.stringify(pageFailure(new Error("Internal private detail"))),
  ).not.toContain("Internal private detail");
  expect(pageFailure(null).title).toBe("This page couldn't open");
});
