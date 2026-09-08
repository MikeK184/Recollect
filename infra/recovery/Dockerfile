# Disposable SFTP transport used only by recovery acceptance tests.
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends openssh-server \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --uid 10001 --create-home --password x recollect \
    && mkdir -p /run/sshd /storage
COPY sshd_config /etc/recollect-sshd.conf
COPY start-sftp.sh /usr/local/bin/start-recovery-sftp
EXPOSE 2222
ENTRYPOINT ["/bin/sh", "/usr/local/bin/start-recovery-sftp"]
