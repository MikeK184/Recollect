FROM recollect-capture-hosts:local
RUN apt-get update && apt-get install -y --no-install-recommends dbus gnome-keyring \
    && rm -rf /var/lib/apt/lists/*
COPY session.sh /usr/local/bin/recollect-test-session
RUN chmod 755 /usr/local/bin/recollect-test-session
