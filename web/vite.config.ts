import { defineConfig } from "vite";
import wasm from "vite-plugin-wasm";

export default defineConfig({
  plugins: [wasm()],
  build: {
    target: "esnext",
    rollupOptions: {
      output: {
        format: "es",
      },
    },
  },
  worker: {
    format: "es",
  },
});
