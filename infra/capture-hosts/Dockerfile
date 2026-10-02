FROM node:22-bookworm-slim AS node
FROM rust:1.94-slim-bookworm
COPY --from=node /usr/local/bin/node /usr/local/bin/node
COPY --from=node /usr/local/lib/node_modules/npm /usr/local/lib/node_modules/npm
RUN ln -s /usr/local/lib/node_modules/npm/bin/npm-cli.js /usr/local/bin/npm
RUN apt-get update && apt-get install -y --no-install-recommends git ca-certificates \
    && rm -rf /var/lib/apt/lists/*
RUN npm install --global --no-audit --no-fund --cache /tmp/recollect-npm \
    @openai/codex@0.154.0 @anthropic-ai/claude-code@2.1.270 @opencode/cli@2.0.21 \
    && rm -rf /tmp/recollect-npm
WORKDIR /workspace
