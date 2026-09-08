FROM rust:1.94-slim-bookworm AS server
WORKDIR /build
ENV CARGO_INCREMENTAL=0 CARGO_PROFILE_RELEASE_DEBUG=0
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
RUN cargo build --locked --release -p recollect-server \
    && target/release/recollect-server openapi > /build/openapi.json

FROM node:22-bookworm-slim AS web
WORKDIR /build/web
COPY web/package.json web/package-lock.json ./
RUN npm ci --no-audit --no-fund
COPY web/ ./
COPY --from=server /build/openapi.json /build/.cache/openapi.json
RUN npm run generate && npm run build

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl openssh-client \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --gid 10001 recollect \
    && useradd --uid 10001 --gid recollect --no-create-home recollect \
    && mkdir -p /var/lib/recollect /opt/recollect/web \
    && chown recollect:recollect /var/lib/recollect
RUN arch="$(dpkg --print-architecture)" \
    && case "$arch" in amd64|arm64) ;; *) exit 1 ;; esac \
    && curl --fail --silent --show-error --location \
      "https://dl.filippo.io/age/v1.3.2?for=linux/$arch" -o /tmp/age.tar.gz \
    && tar -xzf /tmp/age.tar.gz -C /usr/local/bin --strip-components=1 age/age age/age-keygen \
    && rm /tmp/age.tar.gz \
    && test "$(age --version)" = v1.3.2
COPY --from=server /build/target/release/recollect-server /usr/local/bin/recollect-server
COPY --from=web /build/web/dist/ /opt/recollect/web/
USER 10001:10001
WORKDIR /opt/recollect
ENV RECOLLECT_BIND=0.0.0.0:8787 RECOLLECT_STATIC_DIR=/opt/recollect/web \
    RECOLLECT_ARTIFACT_DIR=/var/lib/recollect/artifacts \
    RECOLLECT_CREDENTIAL_FILE=/var/lib/recollect/account-credentials.json \
    RECOLLECT_ERASURE_JOURNAL=/var/lib/recollect/erasure-journal \
    RECOLLECT_MCP_OUTBOX_DIR=/var/lib/recollect/mcp-receipts
EXPOSE 8787
STOPSIGNAL SIGTERM
ENTRYPOINT ["recollect-server"]
CMD ["serve"]
