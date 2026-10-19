FROM rust:1.94-slim-bookworm@sha256:cf9dd0ec73e75f827fe59123fff9dc65af1a1c8363c3c31ee8d7f8ad0b6a5fb2 AS native
WORKDIR /build
ENV CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
RUN cargo build --locked -p recollect-agent --bin recollect-agent --bin recollect-mcp-runner \
    && mkdir /native && cp target/debug/recollect-agent target/debug/recollect-mcp-runner /native/

FROM node:22-bookworm-slim@sha256:c3de60bf2f9dd0ac6370e6117950ff62d6e339527e7472301c9c78a017978392 AS filesystem
WORKDIR /opt/filesystem
RUN npm install --no-audit --no-fund --save-exact @modelcontextprotocol/server-filesystem@2026.8.31

FROM ubuntu:24.04@sha256:68ee22004a9109b44973313b02ff7b1de250f1de0f5ecb628d73f579aaff33ac
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
