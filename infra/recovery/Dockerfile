# Disposable SFTP transport used only by recovery acceptance tests.
FROM debian:bookworm-slim@sha256:7c7b2c966bc9ee8cedfeef67e0e279108992c77681fa595db4a9d65c06ccc587
RUN apt-get update && apt-get install -y --no-install-recommends openssh-server \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --uid 10001 --create-home --password x recollect \
    && mkdir -p /run/sshd /storage
COPY sshd_config /etc/recollect-sshd.conf
COPY start-sftp.sh /usr/local/bin/start-recovery-sftp
EXPOSE 2222
ENTRYPOINT ["/bin/sh", "/usr/local/bin/start-recovery-sftp"]
