#!/usr/bin/env sh
set -eu

if [ -z "${R2_BUCKET:-}" ] || [ -z "${R2_ENDPOINT_URL:-}" ]; then
  echo "R2_BUCKET and R2_ENDPOINT_URL are required"
  exit 1
fi

if [ -z "${BACKUP_DATE:-}" ]; then
  echo "BACKUP_DATE (YYYY-MM-DD) is required"
  exit 1
fi

ENC_FILE="/tmp/backup-${BACKUP_DATE}.sql.gz.enc"
RESTORE_FILE="/tmp/backup-${BACKUP_DATE}.sql.gz"

aws s3 cp "s3://${R2_BUCKET}/db-backups/${BACKUP_DATE}.sql.gz.enc" "${ENC_FILE}" --endpoint-url "${R2_ENDPOINT_URL}"

if [ -z "${BACKUP_KEY:-}" ]; then
  echo "BACKUP_KEY is required for decryption"
  exit 1
fi

openssl enc -d -aes-256-gcm -salt -pbkdf2 -iter 200000 -in "${ENC_FILE}" -out "${RESTORE_FILE}" -pass env:BACKUP_KEY

gunzip -f "${RESTORE_FILE}"
