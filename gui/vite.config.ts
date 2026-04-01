import { defineConfig } from "vite";
import solidPlugin from "vite-plugin-solid";
import devtools from "solid-devtools/vite";
import tailwindcss from "@tailwindcss/vite";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { parse } from "smol-toml";
import { platform, homedir } from "node:os";

const _dir = dirname(fileURLToPath(import.meta.url));

function getOsConfigPath(): string | null {
  const p = platform();
  const home = homedir();
  if (p === "darwin") {
    return join(home, "Library", "Application Support", "dwata", "config.toml");
  } else if (p === "linux") {
    return join(home, ".config", "dwata", "config.toml");
  } else if (p === "win32") {
    return join(process.env.APPDATA || join(home, "AppData", "Roaming"), "dwata", "config.toml");
  }
  return null;
}

function readConfigToml(): Record<string, any> {
  // Check local project paths first (for development)
  for (const rel of ["../config.toml", "../../config.toml"]) {
    try {
      return parse(readFileSync(join(_dir, rel), "utf-8"));
    } catch {
      // config.toml not found at this path, try next
    }
  }
  // Then check OS user config dir (for deployed apps or if no local config)
  const osPath = getOsConfigPath();
  if (osPath) {
    try {
      return parse(readFileSync(osPath, "utf-8"));
    } catch {
      // OS config not found either
    }
  }
  return {};
}

const conf = readConfigToml();
const backendPort: number = (conf?.server as any)?.port ?? 8080;
const guiPort: number = (conf?.gui as any)?.port ?? 3030;

export default defineConfig({
  plugins: [devtools(), tailwindcss(), solidPlugin()],
  server: {
    port: guiPort,
    strictPort: true,
    proxy: {
      "/api": `http://127.0.0.1:${backendPort}`,
    },
  },
  build: {
    target: "esnext",
  },
});
