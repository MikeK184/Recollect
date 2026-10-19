FROM ubuntu:24.04
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates python3 socat dbus gnome-keyring git \
    && rm -rf /var/lib/apt/lists/*
COPY peer.py /opt/recollect-fixture/peer.py
COPY session.sh /opt/recollect-fixture/session.sh
ENTRYPOINT ["dbus-run-session", "--", "bash", "/opt/recollect-fixture/session.sh"]
