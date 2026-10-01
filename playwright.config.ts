import { defineConfig, devices } from "@playwright/test";

const port = Number(process.env.ARCANIA_E2E_PORT ?? "5173");
if (!Number.isInteger(port) || port < 1 || port > 65_535) {
  throw new Error("ARCANIA_E2E_PORT must be an integer TCP port from 1 to 65535.");
}
const baseURL = `http://127.0.0.1:${port}`;

export default defineConfig({
  testDir: "./e2e",
  timeout: 15_000,
  expect: {
    timeout: 2_000,
  },
  fullyParallel: true,
  reporter: "list",
  use: {
    baseURL,
    trace: "on-first-retry",
  },
  webServer: {
    command: `bun run --cwd frontend dev --host 127.0.0.1 --port ${port} --strictPort`,
    env: {
      VITE_DISABLE_BACKEND_PROXY: "1",
    },
    url: baseURL,
    reuseExistingServer: false,
    timeout: 30_000,
  },
  projects: [
    {
      name: "chromium",
      use: { ...devices["Desktop Chrome"] },
    },
  ],
});
