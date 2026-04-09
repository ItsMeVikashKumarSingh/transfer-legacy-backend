---
title: "Backup and Restore Runbook"
date: "2026-04-08"
status: "draft"
---

## Backup Job
Script: `infra/scripts/backup.sh`

## Required Environment
- `DATABASE_URL`
- `OPENBAO_ADDR`
- `OPENBAO_TOKEN`
- `OPENBAO_BACKUP_KEY_PATH` (default `secret/data/transfer-legacy/backup`)
- `OPENBAO_BACKUP_KEY_FIELD` (default `key`)
- `BACKBLAZE_B2_KEY_ID`
- `BACKBLAZE_B2_APP_KEY`
- `BACKBLAZE_B2_ENDPOINT_URL`
- `BACKBLAZE_B2_BACKUP_BUCKET_NAME`

## Backup Procedure
1. Run nightly via cron.
2. Create compressed `pg_dump`.
3. Pull backup encryption key from OpenBao KV at runtime.
4. Encrypt with AES-256-GCM using runtime key material.
5. Upload encrypted artifact to `daily/`.
6. On day 1 of each month, also upload to `monthly/`.
7. Prune daily to 30 objects and monthly to 12 objects.

## Restore Test
Script: `infra/scripts/restore-test.sh`

## Restore Procedure
1. Download dated encrypted backup from B2.
2. Resolve key from OpenBao KV and decrypt backup.
3. Restore into ephemeral Postgres.
4. Validate non-system table count is non-zero.
5. Re-apply all migrations from `migrations/*.sql`.
6. Verify table count does not regress.

## Retention
- Keep 30 daily backups.
- Keep 12 monthly backups.

## Automation
- GitHub workflow: `.github/workflows/backup-restore.yml`
- Daily backup cron: `0 1 * * *`
- Weekly restore validation cron: `0 4 * * 0`
