export function pageFailure(error: unknown) {
  const message = error instanceof Error ? error.message : "";
  const asset =
    /Failed to fetch dynamically imported module|Importing a module script failed|error loading dynamically imported module|Loading chunk [\w-]+ failed/i.test(
      message,
    );
  return asset
    ? {
        title: "This page needs an update",
        description:
          "The app may have been updated while this tab was open. Reload to load the current page. If it still fails, check your connection and try again.",
      }
    : {
        title: "This page couldn't open",
        description: "Reload the page to try again. Your saved work is kept.",
      };
}
