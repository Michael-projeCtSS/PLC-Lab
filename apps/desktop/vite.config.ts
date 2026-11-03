import { defineConfig, type Plugin } from "vite";
import react from "@vitejs/plugin-react";
import { createHash } from "node:crypto";
import { realpath } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { readPreviewContent } from "./preview-content.ts";

function previewContent(): Plugin {
  return {
    name: "plc-lab-read-only-ui-preview",
    apply: "serve",
    async configureServer(server) {
      const repository = await realpath(
        fileURLToPath(new URL("../../", import.meta.url)),
      );
      const pack = fileURLToPath(
        new URL("../../knowledge/PLC_Lab_Knowledge_Pack_v1/", import.meta.url),
      );
      const rootId = createHash("sha256")
        .update(repository.toLowerCase())
        .digest("hex");
      let browserPageLoads = 0;
      server.middlewares.use((request, response, next) => {
        const route = request.url?.split("?")[0];
        if (
          route === "/" &&
          /(?:Chrome|Edg|Firefox)\//.test(request.headers["user-agent"] ?? "")
        )
          browserPageLoads++;
        if (
          route !== "/__plc_lab_preview__/status" &&
          route !== "/__plc_lab_preview__/content"
        )
          return next();
        response.setHeader("Content-Type", "application/json; charset=utf-8");
        response.setHeader("Cache-Control", "no-store");
        if (request.method !== "GET") {
          response.statusCode = 405;
          response.end(JSON.stringify({ error: "Read-only UI preview." }));
          return;
        }
        if (route.endsWith("/status")) {
          response.end(
            JSON.stringify({
              application: "PLC Lab UI preview",
              rootId,
              pid: process.pid,
              managed: !!process.env.PLC_LAB_WEB_PREVIEW_SESSION,
              browserPageLoads,
            }),
          );
          return;
        }
        void readPreviewContent(pack).then(
          (content) => response.end(JSON.stringify(content)),
          (error: unknown) => {
            response.statusCode = 503;
            response.end(
              JSON.stringify({
                error:
                  error instanceof Error
                    ? error.message
                    : "Preview content unavailable.",
              }),
            );
          },
        );
      });
      server.watcher.add(pack);
      server.watcher.on("change", (file) => {
        if (file.startsWith(pack) && file.endsWith(".json"))
          server.ws.send({ type: "full-reload" });
      });
    },
  };
}

export default defineConfig({
  plugins: [react(), previewContent()],
  clearScreen: false,
  server: { host: "127.0.0.1", port: 1420, strictPort: true },
  envPrefix: ["VITE_", "TAURI_ENV_*"],
  build: { target: "es2022" },
});
