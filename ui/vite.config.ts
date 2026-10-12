/// <reference types="vitest/config" />
import { defineConfig } from "vite";

// Tauri forventer en fast port, og trenger ikke at Vite tømmer terminalen.
export default defineConfig({
  clearScreen: false,
  server: { port: 1420, strictPort: true, watch: { ignored: ["**/src-tauri/**"] } },
  envPrefix: ["VITE_", "TAURI_ENV_"],
  build: {
    target: "safari15",
    outDir: "dist",
    emptyOutDir: true,
    // Vinduet med slidene på den andre skjermen i talervisningen er en egen side.
    // Brukerveiledningen (Hjelp → Brukerveiledning) er også et eget vindu.
    rollupOptions: { input: { main: "index.html", audience: "audience.html", help: "help.html" } },
  },
  // `npm test`: testene i test/, med en enkel DOM (happy-dom).
  test: { environment: "happy-dom", include: ["test/**/*.test.ts"] },
});
