import { defineConfig } from "vite";
import wasm from "vite-plugin-wasm";

export default defineConfig({
  base: '/orient-stl/',
  plugins: [wasm()],
  define: {
    __COMMIT_HASH__: JSON.stringify('f5a343b'),
  },
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
