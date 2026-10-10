import { defineConfig } from "vite";

// Tauri forventer en fast port, og trenger ikke at Vite tømmer terminalen.
export default defineConfig({
  clearScreen: false,
  server: { port: 1420, strictPort: true, watch: { ignored: ["**/src-tauri/**"] } },
  envPrefix: ["VITE_", "TAURI_ENV_"],
  build: { target: "safari15", outDir: "dist", emptyOutDir: true },
});
