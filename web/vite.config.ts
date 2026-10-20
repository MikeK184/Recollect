import { defineConfig, type ProxyOptions } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

const api = "http://127.0.0.1:8787";
// The API accepts writes only from its public origin. Requests that come
// from this dev server's own pages are forwarded as that origin; any other
// origin passes through unchanged and is still refused by the server.
const devOrigins = /^http:\/\/(127\.0\.0\.1|localhost):\d+$/;
const proxy: ProxyOptions = {
  target: api,
  configure: (server) => {
    server.on("proxyReq", (request, incoming) => {
      const origin = incoming.headers.origin;
      const host = incoming.headers.host;
      if (origin && host && devOrigins.test(origin) && origin.endsWith(host))
        request.setHeader("origin", api);
    });
  },
};

export default defineConfig({
  plugins: [react(), tailwindcss()],
  server: {
    proxy: { "/api": proxy, "/health": proxy },
  },
  build: { chunkSizeWarningLimit: 700 },
});
