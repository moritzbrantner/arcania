import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

const backendProxyEnabled = process.env.VITE_DISABLE_BACKEND_PROXY !== "1";
const deploymentBase = process.env.VITE_BASE_PATH ?? "/";

export default defineConfig({
  base: deploymentBase,
  plugins: [react()],
  server: {
    port: 5173,
    proxy: backendProxyEnabled
      ? {
          "/api": {
            target: "http://127.0.0.1:4000",
            ws: true,
          },
        }
      : undefined,
  },
});
