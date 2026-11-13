---
title: "Backup and Restore Runbook (IDrive e2)"
date: "2026-11-13"
status: "active"
---

## 1. Architecture Overview
Transfer Legacy database backups are executed daily, encrypted with AES-256 (PBKDF2, 100,000 iterations), and uploaded exclusively to dedicated IDrive e2 S3 storage (`tl-db-backup`). Restores are verified automatically every Sunday in an isolated Docker sandbox.

## 2. Infrastructure & Endpoints
- **Storage Provider:** IDrive e2 (S3 API)
- **Bucket:** `tl-db-backup`
- **Region:** `ap-southeast-2` (`https://s3.ap-southeast-2.idrivee2.com`)
- **Health Monitoring:** Healthchecks.io (`https://hc-ping.com/0e37897e-1d9c-4f52-8750-52195664f3f5`)
- **Database Engine:** Supabase PostgreSQL 17 (Session Pooler Port `5432`)

## 3. Daily Backup Pipeline
- **Workflow:** `.github/workflows/backup.yml`
- **Script:** `infra/scripts/backup-idrive.sh`
- **Schedule:** Everyday at 02:00 UTC (07:30 AM IST)
- **Steps:**
  1. Sends heartbeat START signal to Healthchecks.io.
  2. Runs `pg_dump` over Session Pooler (port 5432) excluding ephemeral connection schemas (`pgbouncer`).
  3. Compresses archive (`tar.gz`) and encrypts via OpenSSL (`AES-256-CBC -pbkdf2 -iter 100000`).
  4. Uploads to `s3://tl-db-backup/transfer-legacy/daily/`.
  5. On the 1st of each month, copies snapshot to `s3://tl-db-backup/transfer-legacy/monthly/`.
  6. Automatically prunes daily backups older than 30 days.
  7. Sends SUCCESS or FAIL signal to Healthchecks.io.

## 4. Weekly Restore Drill Sandbox
- **Workflow:** `.github/workflows/restore-drill.yml`
- **Script:** `infra/scripts/restore-drill-idrive.sh`
- **Schedule:** Every Sunday at 04:00 UTC
- **Steps:**
  1. Downloads latest backup archive from `s3://tl-db-backup/transfer-legacy/daily/`.
  2. Decrypts package using `BACKUP_ENCRYPT_PASSWORD`.
  3. Launches ephemeral `postgres:17-alpine` Docker container.
  4. Pre-provisions Supabase platform schemas/roles and Transfer Legacy domain schemas (`auth_ext`, `core`, `inheritance`, `audit`, `ops`, `notify`, `app`).
  5. Restores dump file.
  6. Asserts table existence and record counts on critical tables:
     - `auth_ext.persons`
     - `core.items`
     - `inheritance.policies`
     - `inheritance.heartbeats`
     - `audit.events`
     - `ops.roles`
     - `app.settings`
  7. Ensures total application tables across Transfer Legacy schemas is `>= 10`.

## 5. Required GitHub Secrets
- `DATABASE_URL`: Session pooler connection string.
- `IDRIVE_ENDPOINT`: `https://s3.ap-southeast-2.idrivee2.com`
- `IDRIVE_BUCKET`: `tl-db-backup`
- `IDRIVE_KEY_ID`: IDrive S3 Access Key ID
- `IDRIVE_SECRET_KEY`: IDrive S3 Secret Access Key
- `BACKUP_ENCRYPT_PASSWORD`: AES-256 PBKDF2 passphrase
- `HEARTBEAT_URL`: `https://hc-ping.com/0e37897e-1d9c-4f52-8750-52195664f3f5`
