#!/usr/bin/env sh
set -eu

required_envs="
BACKBLAZE_B2_BACKUP_BUCKET_NAME
BACKBLAZE_B2_ENDPOINT_URL
BACKBLAZE_B2_KEY_ID
BACKBLAZE_B2_APP_KEY
OPENBAO_ADDR
OPENBAO_TOKEN
"

for env_name in $required_envs; do
  eval "env_value=\${$env_name:-}"
  if [ -z "$env_value" ]; then
    echo "$env_name is required"
    exit 1
  fi
done

BACKUP_DATE="${BACKUP_DATE:-$(date -u +%Y-%m-%d)}"
RESTORE_CONTAINER="${RESTORE_CONTAINER:-tl-restore-test}"
RESTORE_DB="${RESTORE_DB:-restore_test}"
RESTORE_USER="${RESTORE_USER:-postgres}"
RESTORE_PASSWORD="${RESTORE_PASSWORD:-postgres}"
OPENBAO_BACKUP_KEY_PATH="${OPENBAO_BACKUP_KEY_PATH:-secret/data/transfer-legacy/backup}"
OPENBAO_BACKUP_KEY_FIELD="${OPENBAO_BACKUP_KEY_FIELD:-key}"

if ! command -v jq >/dev/null 2>&1; then
  echo "jq is required"
  exit 1
fi

ENC_FILE="/tmp/backup-${BACKUP_DATE}.sql.gz.enc"
RESTORE_FILE="/tmp/backup-${BACKUP_DATE}.sql.gz"
RESTORE_SQL="/tmp/backup-${BACKUP_DATE}.sql"

AWS_ACCESS_KEY_ID="${BACKBLAZE_B2_KEY_ID}" AWS_SECRET_ACCESS_KEY="${BACKBLAZE_B2_APP_KEY}" \
  aws s3 cp "s3://${BACKBLAZE_B2_BACKUP_BUCKET_NAME}/daily/${BACKUP_DATE}.sql.gz.enc" "${ENC_FILE}" --endpoint-url "${BACKBLAZE_B2_ENDPOINT_URL}"

BACKUP_KEY_VALUE=$(curl -sSf \
  -H "X-Vault-Token: ${OPENBAO_TOKEN}" \
  "${OPENBAO_ADDR}/v1/${OPENBAO_BACKUP_KEY_PATH}" \
  | jq -r ".data.data.${OPENBAO_BACKUP_KEY_FIELD}")

if [ -z "${BACKUP_KEY_VALUE}" ] || [ "${BACKUP_KEY_VALUE}" = "null" ]; then
  echo "unable to resolve backup key"
  exit 1
fi

BACKUP_KEY="${BACKUP_KEY_VALUE}" \
  openssl enc -d -aes-256-gcm -salt -pbkdf2 -iter 200000 \
  -in "${ENC_FILE}" \
  -out "${RESTORE_FILE}" \
  -pass env:BACKUP_KEY

gunzip -f "${RESTORE_FILE}"

docker rm -f "${RESTORE_CONTAINER}" >/dev/null 2>&1 || true
docker run -d --name "${RESTORE_CONTAINER}" \
  -e POSTGRES_PASSWORD="${RESTORE_PASSWORD}" \
  -e POSTGRES_DB="${RESTORE_DB}" \
  -p 55432:5432 \
  postgres:16-alpine >/dev/null

cleanup() {
  docker rm -f "${RESTORE_CONTAINER}" >/dev/null 2>&1 || true
}
trap cleanup EXIT INT TERM

sleep 10

cat "${RESTORE_SQL}" | docker exec -i "${RESTORE_CONTAINER}" psql -U "${RESTORE_USER}" -d "${RESTORE_DB}" >/dev/null

BASELINE_COUNT=$(docker exec -i "${RESTORE_CONTAINER}" psql -U "${RESTORE_USER}" -d "${RESTORE_DB}" -Atc "SELECT COUNT(*) FROM information_schema.tables WHERE table_schema NOT IN ('pg_catalog', 'information_schema');")
if [ "${BASELINE_COUNT}" -le 0 ]; then
  echo "restore validation failed: no application tables restored"
  exit 1
fi

for migration in migrations/*.sql; do
  docker exec -i "${RESTORE_CONTAINER}" psql -U "${RESTORE_USER}" -d "${RESTORE_DB}" -v ON_ERROR_STOP=1 -f - >/dev/null < "${migration}"
done

POST_MIGRATION_COUNT=$(docker exec -i "${RESTORE_CONTAINER}" psql -U "${RESTORE_USER}" -d "${RESTORE_DB}" -Atc "SELECT COUNT(*) FROM information_schema.tables WHERE table_schema NOT IN ('pg_catalog', 'information_schema');")
if [ "${POST_MIGRATION_COUNT}" -lt "${BASELINE_COUNT}" ]; then
  echo "restore validation failed: table count regressed after migrations"
  exit 1
fi

echo "restore-test passed for ${BACKUP_DATE}"
