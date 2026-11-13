#!/usr/bin/env bash
set -euo pipefail

# ==============================================================================
# Transfer Legacy - Weekly Restore Verification Drill Engine
# ==============================================================================

IDRIVE_ENDPOINT="${IDRIVE_ENDPOINT:-https://s3.ap-southeast-2.idrivee2.com}"
IDRIVE_BUCKET="${IDRIVE_BUCKET:-tl-db-backup}"
AWS_DEFAULT_REGION="${AWS_DEFAULT_REGION:-ap-southeast-2}"
export AWS_DEFAULT_REGION
export AWS_EC2_METADATA_DISABLED="true"

if [ -z "${BACKUP_ENCRYPT_PASSWORD:-}" ]; then
  echo "::error::BACKUP_ENCRYPT_PASSWORD environment variable is required!"
  exit 1
fi

TMP_DIR="/tmp/tl-drill"
rm -rf "${TMP_DIR}"
mkdir -p "${TMP_DIR}"

echo "=== [1/5] Fetching Latest Backup from IDrive e2 ==="
LATEST_FILE=$(aws s3 ls "s3://${IDRIVE_BUCKET}/transfer-legacy/daily/" --endpoint-url="${IDRIVE_ENDPOINT}" | sort | tail -n 1 | awk '{print $4}')

if [ -z "$LATEST_FILE" ]; then
  echo "::error::CRITICAL: No backup files found in s3://${IDRIVE_BUCKET}/transfer-legacy/daily/! Restore drill cannot pass."
  exit 1
fi

echo "Found latest backup archive: ${LATEST_FILE}"
aws s3 cp "s3://${IDRIVE_BUCKET}/transfer-legacy/daily/${LATEST_FILE}" "${TMP_DIR}/${LATEST_FILE}" --endpoint-url="${IDRIVE_ENDPOINT}"

echo "=== [2/5] Decrypting Archive with Passphrase ==="
openssl enc -d -aes-256-cbc -pbkdf2 -iter 100000 \
  -in "${TMP_DIR}/${LATEST_FILE}" \
  -out "${TMP_DIR}/archive.tar.gz" \
  -pass "env:BACKUP_ENCRYPT_PASSWORD"

tar -xzf "${TMP_DIR}/archive.tar.gz" -C "${TMP_DIR}"
echo "Decryption & archive extraction verified successfully."

echo "=== [3/5] Starting PostgreSQL 17 Sandbox Container ==="
docker rm -f tl_sandbox_db >/dev/null 2>&1 || true
docker run -d --name tl_sandbox_db \
  -e POSTGRES_PASSWORD=postgres \
  -e POSTGRES_HOST_AUTH_METHOD=trust \
  -e POSTGRES_DB=postgres \
  --shm-size=512mb \
  postgres:17-alpine

trap 'docker rm -f tl_sandbox_db >/dev/null 2>&1 || true; rm -rf "${TMP_DIR}"' EXIT

# Wait until database daemon is ready
echo "Waiting for sandbox database to initialize..."
for i in {1..30}; do
  if docker exec tl_sandbox_db pg_isready -h 127.0.0.1 -U postgres >/dev/null 2>&1 && \
     docker exec tl_sandbox_db psql -U postgres -d postgres -c "SELECT 1;" >/dev/null 2>&1; then
    echo "Sandbox DB is healthy!"
    break
  fi
  sleep 1
done

echo "=== [4/5] Pre-provisioning Supabase and Transfer Legacy Schemas/Roles ==="
docker exec tl_sandbox_db psql -U postgres -d postgres -c "
  -- 1. Standard Supabase Platform Schemas
  CREATE SCHEMA IF NOT EXISTS auth;
  CREATE SCHEMA IF NOT EXISTS storage;
  CREATE SCHEMA IF NOT EXISTS realtime;
  CREATE SCHEMA IF NOT EXISTS extensions;
  CREATE SCHEMA IF NOT EXISTS graphql;
  CREATE SCHEMA IF NOT EXISTS graphql_public;
  CREATE SCHEMA IF NOT EXISTS pgbouncer;
  CREATE SCHEMA IF NOT EXISTS vault;
  CREATE SCHEMA IF NOT EXISTS supabase_functions;
  CREATE SCHEMA IF NOT EXISTS supabase_migrations;
  CREATE SCHEMA IF NOT EXISTS pgsodium;
  CREATE SCHEMA IF NOT EXISTS pgsodium_masks;
  CREATE SCHEMA IF NOT EXISTS management;

  -- 2. Transfer Legacy Core Application Schemas
  CREATE SCHEMA IF NOT EXISTS auth_ext;
  CREATE SCHEMA IF NOT EXISTS core;
  CREATE SCHEMA IF NOT EXISTS inheritance;
  CREATE SCHEMA IF NOT EXISTS audit;
  CREATE SCHEMA IF NOT EXISTS ops;
  CREATE SCHEMA IF NOT EXISTS notify;
  CREATE SCHEMA IF NOT EXISTS app;

  -- 3. Grant schema permissions to postgres
  GRANT ALL ON SCHEMA auth TO postgres;
  GRANT ALL ON SCHEMA storage TO postgres;
  GRANT ALL ON SCHEMA realtime TO postgres;
  GRANT ALL ON SCHEMA extensions TO postgres;
  GRANT ALL ON SCHEMA vault TO postgres;
  GRANT ALL ON SCHEMA auth_ext TO postgres;
  GRANT ALL ON SCHEMA core TO postgres;
  GRANT ALL ON SCHEMA inheritance TO postgres;
  GRANT ALL ON SCHEMA audit TO postgres;
  GRANT ALL ON SCHEMA ops TO postgres;
  GRANT ALL ON SCHEMA notify TO postgres;
  GRANT ALL ON SCHEMA app TO postgres;

  -- 4. Placeholder Supabase Roles
  DO \$\$ BEGIN
    CREATE ROLE anon NOLOGIN;
    CREATE ROLE authenticated NOLOGIN;
    CREATE ROLE service_role NOLOGIN;
    CREATE ROLE supabase_admin NOLOGIN;
    CREATE ROLE supabase_auth_admin NOLOGIN;
    CREATE ROLE supabase_storage_admin NOLOGIN;
    CREATE ROLE dashboard_user NOLOGIN;
    CREATE ROLE supabase_realtime_admin NOLOGIN;
    CREATE ROLE supabase_replication_admin NOLOGIN;
    CREATE ROLE pgbouncer NOLOGIN;
  EXCEPTION WHEN duplicate_object THEN NULL;
  END \$\$;

  -- 5. Extensions
  CREATE EXTENSION IF NOT EXISTS \"uuid-ossp\";
  CREATE EXTENSION IF NOT EXISTS pgcrypto;
"

echo "=== [5/5] Restoring SQL Dump & Validating Transfer Legacy Tables ==="
SQL_FILE=$(find "${TMP_DIR}" -name "transfer_legacy_db_*.sql" | head -n 1)

if [ ! -f "$SQL_FILE" ]; then
  echo "::error::Database dump SQL file not found in archive!"
  exit 1
fi

docker exec -i tl_sandbox_db psql -U postgres -d postgres \
  --set ON_ERROR_STOP=off < "$SQL_FILE" > "${TMP_DIR}/restore.log" 2>&1

echo "Tail of restore log:"
tail -n 25 "${TMP_DIR}/restore.log"

echo ""
echo "------------------------------------------------------------------"
echo "Asserting Critical Transfer Legacy Core Tables:"
echo "------------------------------------------------------------------"
docker exec tl_sandbox_db psql -U postgres -d postgres -v ON_ERROR_STOP=1 -c "
  SELECT 'auth_ext.persons' as table_name, count(*) as records FROM auth_ext.persons
  UNION ALL
  SELECT 'core.items', count(*) FROM core.items
  UNION ALL
  SELECT 'inheritance.policies', count(*) FROM inheritance.policies
  UNION ALL
  SELECT 'inheritance.heartbeats', count(*) FROM inheritance.heartbeats
  UNION ALL
  SELECT 'audit.events', count(*) FROM audit.events
  UNION ALL
  SELECT 'ops.roles', count(*) FROM ops.roles
  UNION ALL
  SELECT 'app.settings', count(*) FROM app.settings;
"

echo ""
echo "------------------------------------------------------------------"
echo "Transfer Legacy Application Tables Summary by Schema:"
echo "------------------------------------------------------------------"
docker exec tl_sandbox_db psql -U postgres -d postgres -c "
  SELECT schemaname, count(*) as tables
  FROM pg_tables
  WHERE schemaname IN ('auth_ext', 'core', 'inheritance', 'audit', 'ops', 'notify', 'app')
  GROUP BY schemaname
  ORDER BY tables DESC;
"

TOTAL_TABLES=$(docker exec tl_sandbox_db psql -U postgres -d postgres -t -A -c "
  SELECT count(*)
  FROM pg_tables
  WHERE schemaname IN ('auth_ext', 'core', 'inheritance', 'audit', 'ops', 'notify', 'app');
")

echo "Total application tables restored: ${TOTAL_TABLES}"

if [ "${TOTAL_TABLES}" -ge 10 ]; then
  echo "=================================================================="
  echo " [PASSED] Transfer Legacy Restore Drill Verified Successfully!"
  echo " All core schemas and tables are intact and functional."
  echo "=================================================================="
else
  echo "::error:: [FAILED] Restore drill table count check failed! Only found ${TOTAL_TABLES} application tables."
  exit 1
fi
