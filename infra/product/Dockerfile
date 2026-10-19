FROM rust:1.94-slim-bookworm@sha256:cf9dd0ec73e75f827fe59123fff9dc65af1a1c8363c3c31ee8d7f8ad0b6a5fb2 AS server
WORKDIR /build
ARG RECOLLECT_BUILD_REVISION=development
ARG RECOLLECT_BUILD_TIME=unknown
ENV RECOLLECT_BUILD_REVISION=$RECOLLECT_BUILD_REVISION RECOLLECT_BUILD_TIME=$RECOLLECT_BUILD_TIME
ENV CARGO_INCREMENTAL=0 CARGO_PROFILE_RELEASE_DEBUG=0
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
RUN cargo build --locked --release -p recollect-server \
    && target/release/recollect-server openapi > /build/openapi.json

FROM node:22-bookworm-slim@sha256:c3de60bf2f9dd0ac6370e6117950ff62d6e339527e7472301c9c78a017978392 AS web
WORKDIR /build/web
COPY web/package.json web/package-lock.json ./
RUN npm ci --no-audit --no-fund
COPY web/ ./
COPY --from=server /build/openapi.json /build/.cache/openapi.json
RUN npm run generate && npm run build

FROM debian:bookworm-slim@sha256:7c7b2c966bc9ee8cedfeef67e0e279108992c77681fa595db4a9d65c06ccc587
ARG RECOLLECT_BUILD_REVISION=development
ARG RECOLLECT_BUILD_TIME=unknown
LABEL org.opencontainers.image.revision=$RECOLLECT_BUILD_REVISION org.opencontainers.image.created=$RECOLLECT_BUILD_TIME
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl openssh-client \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --gid 10001 recollect \
    && useradd --uid 10001 --gid recollect --no-create-home recollect \
    && mkdir -p /var/lib/recollect /opt/recollect/web \
    && chown recollect:recollect /var/lib/recollect
# age v1.3.2 publishes no checksum file (only transparency-log .proof files), so the
# release tarball SHA-256 values below are pinned here; they match the digests GitHub
# reports for the release assets. Update all three ARGs when changing the version.
ARG AGE_VERSION=v1.3.2
ARG AGE_SHA256_AMD64=cbe24006683f8eb669266162894b9a522a1af52f2665fbc63a4bb032ed26ac10
ARG AGE_SHA256_ARM64=6b8dc4333c53a5a57c9e5834e3a48f92605d7154014cd07269ff3327db5d37f4
RUN arch="$(dpkg --print-architecture)" \
    && case "$arch" in \
      amd64) expected="$AGE_SHA256_AMD64" ;; \
      arm64) expected="$AGE_SHA256_ARM64" ;; \
      *) exit 1 ;; \
    esac \
    && curl --fail --silent --show-error --location \
      "https://github.com/FiloSottile/age/releases/download/$AGE_VERSION/age-$AGE_VERSION-linux-$arch.tar.gz" -o /tmp/age.tar.gz \
    && echo "$expected  /tmp/age.tar.gz" | sha256sum --check --strict - \
    && tar -xzf /tmp/age.tar.gz -C /usr/local/bin --strip-components=1 age/age age/age-keygen \
    && rm /tmp/age.tar.gz \
    && test "$(age --version)" = "$AGE_VERSION"
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
