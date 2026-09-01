import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

// O Tauri serve estes arquivos do disco; caminhos precisam ser relativos.
export default defineConfig({
  plugins: [react(), tailwindcss()],
  base: "./",
  clearScreen: false,
  server: { port: 5173, strictPort: true },
  build: {
    target: "chrome110", // WebView2 no Windows 10/11
    outDir: "dist",
    emptyOutDir: true,
  },
});
