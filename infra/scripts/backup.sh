#!/usr/bin/env sh
set -eu

required_envs="
DATABASE_URL
OPENBAO_ADDR
OPENBAO_TOKEN
BACKBLAZE_B2_BACKUP_BUCKET_NAME
BACKBLAZE_B2_KEY_ID
BACKBLAZE_B2_APP_KEY
BACKBLAZE_B2_ENDPOINT_URL
"

for env_name in $required_envs; do
  eval "env_value=\${$env_name:-}"
  if [ -z "$env_value" ]; then
    echo "$env_name is required"
    exit 1
  fi
done

OPENBAO_BACKUP_KEY_PATH="${OPENBAO_BACKUP_KEY_PATH:-secret/data/transfer-legacy/backup}"
OPENBAO_BACKUP_KEY_FIELD="${OPENBAO_BACKUP_KEY_FIELD:-key}"

fetch_backup_key() {
  if [ -n "${BACKUP_KEY:-}" ]; then
    printf '%s' "${BACKUP_KEY}"
    return 0
  fi

  if ! command -v jq >/dev/null 2>&1; then
    echo "jq is required to parse OpenBao response when BACKUP_KEY is not set"
    exit 1
  fi

  response=$(curl -sSf \
    -H "X-Vault-Token: ${OPENBAO_TOKEN}" \
    "${OPENBAO_ADDR}/v1/${OPENBAO_BACKUP_KEY_PATH}")
  printf '%s' "${response}" | jq -r ".data.data.${OPENBAO_BACKUP_KEY_FIELD}"
}

BACKUP_KEY_VALUE=$(fetch_backup_key)
if [ -z "${BACKUP_KEY_VALUE}" ] || [ "${BACKUP_KEY_VALUE}" = "null" ]; then
  echo "unable to resolve backup key"
  exit 1
fi

BACKUP_DATE=$(date -u +%Y-%m-%d)
DUMP_FILE="/tmp/backup-${BACKUP_DATE}.sql.gz"
ENC_FILE="/tmp/backup-${BACKUP_DATE}.sql.gz.enc"

pg_dump "${DATABASE_URL}" | gzip > "${DUMP_FILE}"

BACKUP_KEY="${BACKUP_KEY_VALUE}" \
  openssl enc -aes-256-gcm -salt -pbkdf2 -iter 200000 \
  -in "${DUMP_FILE}" \
  -out "${ENC_FILE}" \
  -pass env:BACKUP_KEY

AWS_ACCESS_KEY_ID="${BACKBLAZE_B2_KEY_ID}" AWS_SECRET_ACCESS_KEY="${BACKBLAZE_B2_APP_KEY}" \
  aws s3 cp "${ENC_FILE}" "s3://${BACKBLAZE_B2_BACKUP_BUCKET_NAME}/daily/${BACKUP_DATE}.sql.gz.enc" --endpoint-url "${BACKBLAZE_B2_ENDPOINT_URL}"

if [ "$(date -u +%d)" = "01" ]; then
  AWS_ACCESS_KEY_ID="${BACKBLAZE_B2_KEY_ID}" AWS_SECRET_ACCESS_KEY="${BACKBLAZE_B2_APP_KEY}" \
    aws s3 cp "${ENC_FILE}" "s3://${BACKBLAZE_B2_BACKUP_BUCKET_NAME}/monthly/${BACKUP_DATE}.sql.gz.enc" --endpoint-url "${BACKBLAZE_B2_ENDPOINT_URL}"
fi

# Daily retention: keep latest 30 files.
AWS_ACCESS_KEY_ID="${BACKBLAZE_B2_KEY_ID}" AWS_SECRET_ACCESS_KEY="${BACKBLAZE_B2_APP_KEY}" \
  aws s3 ls "s3://${BACKBLAZE_B2_BACKUP_BUCKET_NAME}/daily/" --endpoint-url "${BACKBLAZE_B2_ENDPOINT_URL}" \
  | awk '{print $4}' \
  | sort -r \
  | awk 'NR>30' \
  | while read -r old_key; do
      if [ -n "${old_key}" ]; then
        AWS_ACCESS_KEY_ID="${BACKBLAZE_B2_KEY_ID}" AWS_SECRET_ACCESS_KEY="${BACKBLAZE_B2_APP_KEY}" \
          aws s3 rm "s3://${BACKBLAZE_B2_BACKUP_BUCKET_NAME}/daily/${old_key}" --endpoint-url "${BACKBLAZE_B2_ENDPOINT_URL}"
      fi
    done

# Monthly retention: keep latest 12 files.
AWS_ACCESS_KEY_ID="${BACKBLAZE_B2_KEY_ID}" AWS_SECRET_ACCESS_KEY="${BACKBLAZE_B2_APP_KEY}" \
  aws s3 ls "s3://${BACKBLAZE_B2_BACKUP_BUCKET_NAME}/monthly/" --endpoint-url "${BACKBLAZE_B2_ENDPOINT_URL}" \
  | awk '{print $4}' \
  | sort -r \
  | awk 'NR>12' \
  | while read -r old_key; do
      if [ -n "${old_key}" ]; then
        AWS_ACCESS_KEY_ID="${BACKBLAZE_B2_KEY_ID}" AWS_SECRET_ACCESS_KEY="${BACKBLAZE_B2_APP_KEY}" \
          aws s3 rm "s3://${BACKBLAZE_B2_BACKUP_BUCKET_NAME}/monthly/${old_key}" --endpoint-url "${BACKBLAZE_B2_ENDPOINT_URL}"
      fi
    done

echo "backup completed for ${BACKUP_DATE}"
