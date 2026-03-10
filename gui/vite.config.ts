import { defineConfig } from "vite";
import solidPlugin from "vite-plugin-solid";
import devtools from "solid-devtools/vite";
import tailwindcss from "@tailwindcss/vite";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { parse } from "smol-toml";

const _dir = dirname(fileURLToPath(import.meta.url));

function readProjectToml(): Record<string, any> {
  for (const rel of ["../project.toml", "../../project.toml"]) {
    try {
      return parse(readFileSync(join(_dir, rel), "utf-8"));
    } catch {
      // project.toml not found at this path, try next
    }
  }
  return {};
}

const conf = readProjectToml();
const backendPort: number = (conf?.server as any)?.port ?? 8080;
const guiPort: number = (conf?.gui as any)?.port ?? 3030;

export default defineConfig({
  plugins: [devtools(), tailwindcss(), solidPlugin()],
  server: {
    port: guiPort,
    proxy: {
      "/api": `http://127.0.0.1:${backendPort}`,
    },
  },
  build: {
    target: "esnext",
  },
});
