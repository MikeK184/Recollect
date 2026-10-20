// Applies the Light or Ink theme before first paint (ADR 0025). An explicit
// choice is a non-sensitive browser preference; otherwise the system setting
// decides. src/design/theme-mode.ts keeps the attribute current afterwards.
(function () {
  var stored = null;
  try {
    stored = window.localStorage.getItem("recollect-theme");
  } catch (error) {
    stored = null;
  }
  var preference = stored === "light" || stored === "dark" ? stored : "system";
  var dark =
    preference === "dark" ||
    (preference === "system" &&
      !!window.matchMedia &&
      window.matchMedia("(prefers-color-scheme: dark)").matches);
  var root = document.documentElement;
  root.setAttribute("data-theme", dark ? "dark" : "light");
  var meta = document.querySelector('meta[name="theme-color"]');
  if (meta) meta.setAttribute("content", dark ? "#0b1018" : "#f4f6f8");
})();
