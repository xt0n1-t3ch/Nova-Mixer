import path from "node:path";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import { svelteTesting } from "@testing-library/svelte/vite";
import { defineConfig } from "vitest/config";

export default defineConfig({
  plugins: [svelte({ hot: false }), svelteTesting()],
  resolve: {
    alias: {
      // The suites live in `../tests`, outside this package, so `@` gives them a
      // stable handle on the frontend source, and the testing library is pinned
      // here because a bare specifier would resolve from the repo root instead.
      "@": path.resolve(__dirname, "src"),
      "@testing-library/svelte": path.resolve(__dirname, "node_modules/@testing-library/svelte"),
      "@testing-library/user-event": path.resolve(
        __dirname,
        "node_modules/@testing-library/user-event",
      ),
      "@testing-library/jest-dom": path.resolve(__dirname, "node_modules/@testing-library/jest-dom"),
    },
  },
  test: {
    environment: "happy-dom",
    globals: true,
    setupFiles: ["../tests/setup.ts"],
    include: [
      "../tests/unit/**/*.test.ts",
      "../tests/integration/**/*.test.ts",
      "../tests/components/**/*.test.ts",
      "../tests/contracts/**/*.test.ts",
    ],
    passWithNoTests: true,
  },
});
