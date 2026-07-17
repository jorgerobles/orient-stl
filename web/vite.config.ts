import { defineConfig } from "vite";
import wasm from "vite-plugin-wasm";
import { execSync } from "child_process";

const commitHash = execSync("git rev-parse --short HEAD").toString().trim();

export default defineConfig({
  base: '/orient-stl/',
  plugins: [wasm()],
  define: {
    __COMMIT_HASH__: JSON.stringify(commitHash),
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
