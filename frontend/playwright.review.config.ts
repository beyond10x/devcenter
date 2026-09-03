import { defineConfig, devices } from "@playwright/test";

const port = process.env.DEVCENTER_REVIEW_E2E_PORT ?? "4174";
const origin = `http://127.0.0.1:${port}`;

export default defineConfig({
  testDir: "./review-e2e",
  fullyParallel: false,
  forbidOnly: Boolean(process.env.CI),
  retries: 0,
  reporter: process.env.CI ? "github" : "list",
  use: {
    baseURL: origin,
    trace: "retain-on-failure",
    viewport: { width: 1440, height: 1000 },
  },
  projects: [{ name: "chromium", use: { ...devices["Desktop Chrome"] } }],
  webServer: {
    command: `pnpm exec vite --mode review --host 127.0.0.1 --port ${port}`,
    url: origin,
    reuseExistingServer: false,
  },
});
