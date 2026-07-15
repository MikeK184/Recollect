import { defineConfig } from "@playwright/test";
export default defineConfig({
  testDir: "./tests",
  workers: 1,
  retries: 0,
  timeout: 45_000,
  outputDir: "../.cache/ui-test-results",
  reporter: "line",
  use: {
    baseURL: process.env.RECOLLECT_UI_TEST_ORIGIN ?? "http://127.0.0.1:8788",
    channel: "chrome",
    viewport: { width: 1440, height: 960 },
    trace: "off",
    video: "off",
    screenshot: "off",
  },
});
