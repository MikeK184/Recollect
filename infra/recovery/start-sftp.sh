#!/bin/sh
set -eu
mkdir -p /home/recollect/.ssh
cp /fixture/client-key.pub /home/recollect/.ssh/authorized_keys
chmod 700 /home/recollect/.ssh
chmod 600 /home/recollect/.ssh/authorized_keys
chown -R 10001:10001 /home/recollect/.ssh
chown 10001:10001 /storage
exec /usr/sbin/sshd -D -e -f /etc/recollect-sshd.conf
