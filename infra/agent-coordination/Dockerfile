FROM ubuntu:24.04@sha256:68ee22004a9109b44973313b02ff7b1de250f1de0f5ecb628d73f579aaff33ac
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates python3 socat dbus gnome-keyring git \
    && rm -rf /var/lib/apt/lists/*
COPY peer.py /opt/recollect-fixture/peer.py
COPY session.sh /opt/recollect-fixture/session.sh
ENTRYPOINT ["dbus-run-session", "--", "bash", "/opt/recollect-fixture/session.sh"]
