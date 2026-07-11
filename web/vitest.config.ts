import { defineConfig } from "vitest/config";

// Minimal test config. Centering math is tested as a pure function — no DOM,
// no three.js renderer — so the default node environment is sufficient.
export default defineConfig({
  test: {
    environment: "node",
    include: ["src/**/*.test.ts"],
  },
});
