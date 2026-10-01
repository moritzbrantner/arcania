import { createServer } from "node:net";
import { defineConfig, devices } from "@playwright/test";

const deployedURL = process.env.ARCANIA_PAGES_URL;
const port = deployedURL ? undefined : process.env.ARCANIA_PAGES_TEST_PORT ? Number(process.env.ARCANIA_PAGES_TEST_PORT) : await new Promise<number>((resolve, reject) => {
  const server = createServer();
  server.once("error", reject);
  server.listen(0, "127.0.0.1", () => {
    const address = server.address();
    if (!address || typeof address === "string") { server.close(); reject(new Error("No test port")); return; }
    server.close(() => resolve(address.port));
  });
});

if (port !== undefined) { process.env.ARCANIA_PAGES_TEST_PORT = String(port); }
const baseURL = deployedURL ?? `http://127.0.0.1:${port}/arcania/`;

export default defineConfig({
  testDir: "./e2e-pages",
  outputDir: "./test-results-pages",
  timeout: 45_000,
  expect: { timeout: 10_000 },
  workers: 1,
  use: { baseURL, trace: "retain-on-failure" },
  webServer: deployedURL ? undefined : {
    command: `bun run --cwd frontend dev:preview --host 127.0.0.1 --port ${port} --strictPort`,
    url: `http://127.0.0.1:${port}/arcania/`,
    reuseExistingServer: false,
    env: { VITE_BASE_PATH: "/arcania/", VITE_DISABLE_BACKEND_PROXY: "1" },
  },
  projects: [
    { name: "desktop", use: { ...devices["Desktop Chrome"] } },
    { name: "mobile", use: { ...devices["iPhone 13"], defaultBrowserType: "chromium" } },
  ],
});
