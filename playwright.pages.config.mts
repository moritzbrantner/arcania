import { createServer } from "node:net";
import { defineConfig, devices } from "@playwright/test";

const port = process.env.ARCANIA_PAGES_TEST_PORT ? Number(process.env.ARCANIA_PAGES_TEST_PORT) : await new Promise<number>((resolve, reject) => {
  const server = createServer();
  server.once("error", reject);
  server.listen(0, "127.0.0.1", () => {
    const address = server.address();
    if (!address || typeof address === "string") { server.close(); reject(new Error("No test port")); return; }
    server.close(() => resolve(address.port));
  });
});

process.env.ARCANIA_PAGES_TEST_PORT = String(port);

export default defineConfig({
  testDir: "./e2e-pages",
  outputDir: "./test-results-pages",
  timeout: 45_000,
  expect: { timeout: 10_000 },
  workers: 1,
  use: { baseURL: `http://127.0.0.1:${port}/arcania/`, trace: "retain-on-failure" },
  webServer: {
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
