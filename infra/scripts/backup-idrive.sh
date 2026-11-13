#!/usr/bin/env bash
set -euo pipefail

# ==============================================================================
# Transfer Legacy - Database Backup Engine (IDrive e2)
# ==============================================================================

IDRIVE_ENDPOINT="${IDRIVE_ENDPOINT:-https://s3.ap-southeast-2.idrivee2.com}"
IDRIVE_BUCKET="${IDRIVE_BUCKET:-tl-db-backup}"
AWS_DEFAULT_REGION="${AWS_DEFAULT_REGION:-ap-southeast-2}"
export AWS_DEFAULT_REGION
export AWS_EC2_METADATA_DISABLED="true"

if [ -z "${DATABASE_URL:-}" ]; then
  echo "::error::DATABASE_URL environment variable is required!"
  exit 1
fi

if [ -z "${BACKUP_ENCRYPT_PASSWORD:-}" ]; then
  echo "::error::BACKUP_ENCRYPT_PASSWORD environment variable is required!"
  exit 1
fi

TIMESTAMP=$(date -u +"%Y%m%d_%H%M%SZ")
DATE_DAY=$(date -u +"%d")
WORK_DIR=$(mktemp -d -t "tl-backup-XXXXXX")
trap 'rm -rf "${WORK_DIR}"' EXIT

DUMP_SQL="${WORK_DIR}/transfer_legacy_db_${TIMESTAMP}.sql"
ARCHIVE_TAR="${WORK_DIR}/transfer_legacy_archive_${TIMESTAMP}.tar.gz"
ENCRYPTED_PKG="${WORK_DIR}/transfer_legacy_backup_${TIMESTAMP}.tar.gz.enc"

echo "=== [1/4] Dumping Transfer Legacy Supabase Database ==="
pg_dump "${DATABASE_URL}" \
  --format=plain \
  --no-owner \
  --no-privileges \
  --quote-all-identifiers \
  --exclude-schema='pgbouncer' \
  --file="${DUMP_SQL}"

DUMP_SIZE=$(du -h "${DUMP_SQL}" | cut -f1)
echo "Dump completed successfully (${DUMP_SIZE})."

echo "=== [2/4] Compressing Archive ==="
tar -czf "${ARCHIVE_TAR}" -C "${WORK_DIR}" "$(basename "${DUMP_SQL}")"
ARCHIVE_SIZE=$(du -h "${ARCHIVE_TAR}" | cut -f1)
echo "Compressed archive: ${ARCHIVE_SIZE}"

echo "=== [3/4] Encrypting (AES-256-CBC PBKDF2) ==="
openssl enc -aes-256-cbc -pbkdf2 -iter 100000 \
  -in "${ARCHIVE_TAR}" \
  -out "${ENCRYPTED_PKG}" \
  -pass "env:BACKUP_ENCRYPT_PASSWORD"

ENC_SIZE=$(du -h "${ENCRYPTED_PKG}" | cut -f1)
echo "Encrypted package size: ${ENC_SIZE}"

echo "=== [4/4] Uploading to IDrive e2: s3://${IDRIVE_BUCKET}/transfer-legacy/daily/ ==="
REMOTE_DAILY="s3://${IDRIVE_BUCKET}/transfer-legacy/daily/$(basename "${ENCRYPTED_PKG}")"
aws s3 cp "${ENCRYPTED_PKG}" "${REMOTE_DAILY}" --endpoint-url="${IDRIVE_ENDPOINT}"

# Monthly snapshot retention on the 1st of each month
if [ "${DATE_DAY}" = "01" ]; then
  REMOTE_MONTHLY="s3://${IDRIVE_BUCKET}/transfer-legacy/monthly/$(basename "${ENCRYPTED_PKG}")"
  echo "Archiving monthly snapshot: ${REMOTE_MONTHLY}"
  aws s3 cp "${ENCRYPTED_PKG}" "${REMOTE_MONTHLY}" --endpoint-url="${IDRIVE_ENDPOINT}"
fi

# Prune daily backups older than 30 files
echo "Enforcing 30-day retention policy in s3://${IDRIVE_BUCKET}/transfer-legacy/daily/..."
aws s3 ls "s3://${IDRIVE_BUCKET}/transfer-legacy/daily/" --endpoint-url="${IDRIVE_ENDPOINT}" \
  | awk '{print $4}' \
  | sort -r \
  | awk 'NR>30' \
  | while read -r old_file; do
      if [ -n "${old_file}" ]; then
        echo "Removing expired daily backup: ${old_file}"
        aws s3 rm "s3://${IDRIVE_BUCKET}/transfer-legacy/daily/${old_file}" --endpoint-url="${IDRIVE_ENDPOINT}"
      fi
    done

echo "=================================================================="
echo "Transfer Legacy backup, encryption, and IDrive e2 upload COMPLETE!"
echo "Uploaded: ${REMOTE_DAILY}"
echo "=================================================================="
