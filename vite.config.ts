import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Port fixe attendu par tauri.conf.json (devUrl)
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: { port: 1420, strictPort: true, watch: { ignored: ["**/src-tauri/**"] } },
  test: { environment: "node" },
});
