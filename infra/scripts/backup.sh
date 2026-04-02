#!/usr/bin/env sh
set -eu

if [ -z "${DATABASE_URL:-}" ] || [ -z "${OPENBAO_ADDR:-}" ] || [ -z "${OPENBAO_TOKEN:-}" ]; then
  echo "DATABASE_URL, OPENBAO_ADDR, and OPENBAO_TOKEN are required"
  exit 1
fi

if [ -z "${R2_BUCKET:-}" ] || [ -z "${R2_ACCESS_KEY:-}" ] || [ -z "${R2_SECRET_KEY:-}" ] || [ -z "${R2_ENDPOINT_URL:-}" ]; then
  echo "R2_BUCKET, R2_ACCESS_KEY, R2_SECRET_KEY, and R2_ENDPOINT_URL are required"
  exit 1
fi

BACKUP_DATE=$(date -u +%Y-%m-%d)
DUMP_FILE="/tmp/backup-${BACKUP_DATE}.sql.gz"
ENC_FILE="/tmp/backup-${BACKUP_DATE}.sql.gz.enc"

pg_dump "${DATABASE_URL}" | gzip > "${DUMP_FILE}"

# Expect OPENBAO to provide BACKUP_KEY via a secure KV lookup in production
if [ -z "${BACKUP_KEY:-}" ]; then
  echo "BACKUP_KEY is required for encryption"
  exit 1
fi

openssl enc -aes-256-gcm -salt -pbkdf2 -iter 200000 -in "${DUMP_FILE}" -out "${ENC_FILE}" -pass env:BACKUP_KEY

aws s3 cp "${ENC_FILE}" "s3://${R2_BUCKET}/db-backups/${BACKUP_DATE}.sql.gz.enc" --endpoint-url "${R2_ENDPOINT_URL}"
