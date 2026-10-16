import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

// The Tauri CLI expects a fixed dev port and reads `dist/` for release builds.
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: { port: 5173, strictPort: true },
  build: { target: "chrome120", outDir: "dist" },
});
