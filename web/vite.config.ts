/// <reference types="vitest/config" />
import path from "node:path";
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import { execSync } from "node:child_process";

function getGitVersion(): string {
  const envVersion = process.env.GIT_VERSION?.trim();
  if (envVersion) return envVersion;
  try {
    return execSync("git describe --tags --exact-match", { encoding: "utf-8", stdio: ["pipe", "pipe", "pipe"] }).trim();
  } catch {
    try {
      return execSync("git rev-parse --short HEAD", { encoding: "utf-8" }).trim();
    } catch {
      return "unknown";
    }
  }
}

// https://vite.dev/config/
export default defineConfig({
  plugins: [react(), tailwindcss()],
  define: {
    __APP_VERSION__: JSON.stringify(getGitVersion()),
  },
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
  },
  test: {
    environment: "jsdom",
  },
});
