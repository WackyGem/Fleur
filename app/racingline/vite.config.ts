import { execFileSync } from "child_process"
import { readFileSync } from "fs"
import path from "path"
import tailwindcss from "@tailwindcss/vite"
import react from "@vitejs/plugin-react"
import { defineConfig } from "vite"

type PackageJson = {
  name: string
  version: string
}

const repositoryRoot = path.resolve(__dirname, "../..")
const packageJson = JSON.parse(
  readFileSync(new URL("./package.json", import.meta.url), "utf8")
) as PackageJson

function gitSha(): string {
  return (
    process.env.RACINGLINE_GIT_SHA ??
    execFileSync("git", ["rev-parse", "--short", "HEAD"], {
      cwd: repositoryRoot,
      encoding: "utf8",
    }).trim()
  )
}

// https://vite.dev/config/
export default defineConfig({
  envDir: "../..",
  plugins: [react(), tailwindcss()],
  define: {
    __RACINGLINE_BUILD_METADATA__: JSON.stringify({
      appName: packageJson.name,
      appVersion: packageJson.version,
      gitSha: gitSha(),
      buildTime: new Date().toISOString(),
    }),
  },
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
  },
})
