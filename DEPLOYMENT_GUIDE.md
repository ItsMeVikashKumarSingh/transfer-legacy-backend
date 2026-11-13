# Transfer Legacy - Serverless Deployment Guide

This guide details the procedure for deploying the **Transfer Legacy** secure inheritance backend using a modern, stateless **Serverless Architecture** via Vercel and Supabase. This approach eliminates the need for maintaining a stateful VPS and an external OpenBao vault, providing immense horizontal scalability and drastically reduced costs.

## Architecture & Pricing Overview

By moving to a serverless architecture, you bypass the fixed overhead costs of traditional VPS and HSM-backed vault deployments.

| Service | Purpose | Estimated Cost |
| --- | --- | --- |
| **Vercel** | Serverless API execution, routing | **$0/mo** (Hobby) or $20/mo (Pro for higher limits) |
| **Supabase** | PostgreSQL Database & `pg_cron` | **$0/mo** (Free) or $25/mo (Pro) |
| **Upstash (Redis)** | Rate limiting, OPAQUE state, caching | **$0/mo** (Free tier covers 10k req/day) |
| **Resend** | Transactional emails (security alerts, invites) | **$0/mo** (Free tier covers 3k emails/mo) |
| **Backblaze B2** | Encrypted blob storage & backups | **$0/mo** (First 10GB free, then $0.006/GB) |
| **Total Base Cost** | | **$0 / month** |

---

## Phase 1: Environment Variables Setup

Since we are running in a stateless serverless environment, configuration and secrets are injected via environment variables.

### 1. Where to add them
You must add these environment variables to your **Vercel Project Settings** (`Settings > Environment Variables`).

### 2. Required Environment Variables

#### Core & Serverless Configuration
- `TL_SERVERLESS`: Must be set to `true` to enable stateless operation and bypass OpenBao.
- `TL_CRON_SECRET`: A secure, random string (e.g., generate via `openssl rand -hex 32`). This protects your background job webhooks.
- `TL_SERVER_PRIVATE_KEY_B64`: Your Ed25519 private key encoded in Base64. This powers the `InMemorySigner` for all cryptographic signatures (replacing OpenBao's Transit engine).

#### Database & Caching
- `DATABASE_URL`: Your Supabase **Transaction Pooler** connection string (usually port `6543`). **Crucial:** You must use the pooler URL to prevent Vercel from exhausting Supabase's direct connection limits.
- `REDIS_URL`: Your Upstash Redis connection string (e.g., `rediss://default:password@region.upstash.io:6379`).

#### Supabase Auth
- `SUPABASE_URL`: Your Supabase Project URL.
- `SUPABASE_SECRET_KEY`: Your Supabase `service_role` secret key.
- `SUPABASE_PUBLISHABLE_KEY`: Your Supabase anon publishable key.

#### Application Secrets
*(These replace the secrets previously fetched dynamically from OpenBao)*
- `SERVER_HMAC_SECRET`: Secure random string for HMAC generation.
- `SERVER_AEAD_KEY`: Secure random string for AEAD encryption.
- `JWT_SECRET`: Secure random string for JWT signing.
- `OPAQUE_SERVER_SETUP`: Your OPAQUE server setup parameters.

#### Integrations
- `RESEND_API_KEY`, `RESEND_FROM_EMAIL`, `RESEND_FROM_NAME`: For email delivery.
- `BACKBLAZE_B2_*`: (Key ID, App Key, Bucket Names, Endpoint URL) for object storage.
- `BACKUP_KEY`: Encryption key for automated database backups.

---

## Phase 2: Database Initialization & Migrations

Before deploying the API, your Supabase PostgreSQL database must be initialized with the correct schema and cron jobs.

1. **Link your Supabase Project:**
   ```bash
   supabase link --project-ref <your-project-ref>
   ```

2. **Run Migrations:**
   Push all migrations located in the `migrations/` folder to your Supabase database.
   ```bash
   supabase db push
   ```

---

## Phase 3: Cron Jobs Configuration

Instead of a persistent polling worker, background tasks (heartbeat evaluation, audit anchoring, conflict resolution) are now stateless webhooks triggered by Supabase's `pg_cron` and `pg_net` extensions.

1. Locate the migration file `0025_serverless_db_cron_scheduler.sql` in your `migrations/` folder.
2. In your Supabase SQL Editor, or via a custom migration, you must configure the cron jobs to point to your live Vercel URL.
3. Update the `target_url` in the `pg_net` HTTP POST requests to point to your Vercel domain (e.g., `https://api.transferlegacy.com/v1/jobs/heartbeat-eval`).
4. Update the `Authorization: Bearer` header in the `pg_net` requests to match the `TL_CRON_SECRET` you set in Vercel.

---

## Phase 4: Vercel Deployment

Deployment to Vercel is streamlined using the `vercel-rust` builder. The routing and build configuration are already defined in the `vercel.json` file at the root of the repository.

1. **Install Vercel CLI:**
   ```bash
   npm i -g vercel
   ```

2. **Deploy to Vercel:**
   Run the deployment command from the repository root:
   ```bash
   vercel --prod
   ```

Vercel will detect the `vercel.json`, compile the Rust application, and deploy it as a serverless function.

---

## Security Reminders for Serverless

- **Environment Variable Security:** Since `TL_SERVER_PRIVATE_KEY_B64` and `SERVER_AEAD_KEY` are stored in Vercel environment variables, ensure that Vercel team access is strictly limited.
- **Webhook Protection:** Never expose your `/v1/jobs/*` endpoints without the `TL_CRON_SECRET` bearer token protection. Supabase will securely pass this token when triggering the cron jobs.
- **Connection Pooling:** Always use the Supabase Transaction Pooler (port `6543`) in serverless environments to prevent connection exhaustion.
