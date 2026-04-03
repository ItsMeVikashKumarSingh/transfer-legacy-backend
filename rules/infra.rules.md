# Infrastructure Rules — Transfer Legacy Backend

Reference sections in `project_detail.md`: deployment, environment separation, observability, backups, and signing.

## Production stack
- Hetzner hosts the backend runtime.
- Supabase provides PostgreSQL and selected platform services.
- Redis handles rate limiting, idempotency, and worker queue support.
- Cloudflare R2 stores encrypted user files.
- OpenBao KV manages application secrets.
- Signing service handles release-record and audit-anchor signatures required by the system design.[file:1]

## Deployment rules
- Backend must run in Docker.
- Containers run as non-root.
- Production services are internal-network only except reverse proxy / API ingress.
- No public Redis, no public DB, no public OpenBao, no public signing service.
- Environments must be fully separated: local, staging, production.

## Config and secrets
- Production secrets come from OpenBao KV, not `.env` files.
- Local development may use `.env.local`, gitignored only.
- No secret lookups inside hot request paths if startup loading is sufficient.
- Rotate credentials on a schedule and after any incident.

## Backups and recovery
- PostgreSQL backups must be scheduled and restore-tested.
- R2 versioning or equivalent recovery path must be enabled for encrypted files.
- Redis is not a source of truth for user data.
- Recovery procedures must be documented and tested.

## Monitoring
- Use Sentry for error tracking.
- Use PostHog for product analytics without leaking sensitive data.
- Use metrics for API latency, worker lag, signature failures, replay failures, queue depth, and any forbidden decrypt-attempt signal.
- Alert immediately on any evidence of attempted server-side decryption or audit-chain inconsistency.[file:1]
