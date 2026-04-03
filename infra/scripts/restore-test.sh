#!/usr/bin/env sh
set -eu

if [ -z "${BACKBLAZE_B2_BUCKET_NAME:-}" ] || [ -z "${BACKBLAZE_B2_ENDPOINT_URL:-}" ]; then
  echo "BACKBLAZE_B2_BUCKET_NAME and BACKBLAZE_B2_ENDPOINT_URL are required"
  exit 1
fi

if [ -z "${BACKUP_DATE:-}" ]; then
  echo "BACKUP_DATE (YYYY-MM-DD) is required"
  exit 1
fi

ENC_FILE="/tmp/backup-${BACKUP_DATE}.sql.gz.enc"
RESTORE_FILE="/tmp/backup-${BACKUP_DATE}.sql.gz"

AWS_ACCESS_KEY_ID="${BACKBLAZE_B2_KEY_ID}" AWS_SECRET_ACCESS_KEY="${BACKBLAZE_B2_APP_KEY}" \
  aws s3 cp "s3://${BACKBLAZE_B2_BUCKET_NAME}/db-backups/${BACKUP_DATE}.sql.gz.enc" "${ENC_FILE}" --endpoint-url "${BACKBLAZE_B2_ENDPOINT_URL}"

if [ -z "${BACKUP_KEY:-}" ]; then
  echo "BACKUP_KEY is required for decryption"
  exit 1
fi

openssl enc -d -aes-256-gcm -salt -pbkdf2 -iter 200000 -in "${ENC_FILE}" -out "${RESTORE_FILE}" -pass env:BACKUP_KEY

gunzip -f "${RESTORE_FILE}"
