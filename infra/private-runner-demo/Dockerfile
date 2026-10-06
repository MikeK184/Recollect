FROM rust:1.94-slim-bookworm AS native
WORKDIR /build
ENV CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
RUN cargo build --locked -p recollect-agent --bin recollect-agent --bin recollect-mcp-runner \
    && mkdir /native && cp target/debug/recollect-agent target/debug/recollect-mcp-runner /native/

FROM node:22-bookworm-slim AS filesystem
WORKDIR /opt/filesystem
RUN npm install --no-audit --no-fund --save-exact @modelcontextprotocol/server-filesystem@2026.8.31

FROM ubuntu:24.04
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates dbus gnome-keyring socat \
    && rm -rf /var/lib/apt/lists/* \
    && mkdir -p /srv/recollect-demo /run/recollect /root/.local/share
COPY --from=filesystem /usr/local/bin/node /usr/local/bin/node
COPY --from=filesystem /opt/filesystem /opt/filesystem
COPY --from=native /native/ /usr/local/bin/
COPY session.sh /usr/local/bin/recollect-demo-session
RUN chmod 755 /usr/local/bin/recollect-demo-session
ENV RECOLLECT_DEVICE_PROFILE=ubuntu-filesystem-demo
ENTRYPOINT ["dbus-run-session", "--", "/usr/local/bin/recollect-demo-session"]
