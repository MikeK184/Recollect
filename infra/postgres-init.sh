#!/usr/bin/env bash
set -euo pipefail
psql --username "$POSTGRES_USER" --dbname "$POSTGRES_DB" \
  --set=app_password="$RECOLLECT_DB_PASSWORD" --set=ON_ERROR_STOP=1 <<'SQL'
SELECT format('CREATE ROLE recollect_app LOGIN PASSWORD %L', :'app_password') \gexec
GRANT CONNECT ON DATABASE recollect TO recollect_app;
GRANT USAGE ON SCHEMA public TO recollect_app;
SQL
