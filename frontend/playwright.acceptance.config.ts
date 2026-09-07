import { defineConfig } from "@playwright/test";

const origin = process.env.DEVCENTER_ORIGIN;
if (!origin || !process.env.DEVCENTER_EVIDENCE_ROOT) {
  throw new Error("Local acceptance requires an origin and evidence directory");
}
const host = new URL(origin).hostname;
if (!host.endsWith(".localhost") && !host.endsWith(".test")) {
  throw new Error("Local acceptance requires a reserved test domain");
}

export default defineConfig({
  testDir: "./acceptance",
  fullyParallel: false,
  workers: 1,
  retries: 0,
  timeout: 420_000,
  outputDir: `${process.env.DEVCENTER_EVIDENCE_ROOT}/playwright`,
  use: {
    baseURL: origin,
    headless: true,
    storageState: process.env.PROJECTS_STORAGE_STATE,
    viewport: { width: 1600, height: 1100 },
    serviceWorkers: "block",
    extraHTTPHeaders: { Origin: origin },
    launchOptions: {
      args: process.env.DEVCENTER_LOCAL_TLS_SPKI
        ? [`--ignore-certificate-errors-spki-list=${process.env.DEVCENTER_LOCAL_TLS_SPKI}`]
        : [],
    },
  },
});
