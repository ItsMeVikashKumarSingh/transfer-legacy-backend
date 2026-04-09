---
title: "Incident Response Runbook"
date: "2026-04-08"
status: "draft"
---

## Severity
- `SEV-1`: data loss risk, release integrity break, decrypt-path evidence
- `SEV-2`: major auth/release outage, queue backlog causing SLA breach
- `SEV-3`: degraded non-critical feature

## Immediate Actions
1. Freeze deploys and announce incident channel.
2. Assign incident commander and scribe.
3. Capture timeframe and impacted components.
4. Rotate exposed credentials if compromise is suspected.
5. For `SEV-1`, disable release endpoints and pause release workers.

## Investigation Checklist
1. Review Sentry events and request IDs.
2. Check Redis queue depth and failed jobs.
3. Verify `audit.events` chain continuity for affected policies.
4. Validate OpenBao transit signing health.
5. Confirm B2 object integrity for impacted attachments/anchors.

## Containment
1. Block abusive IPs at edge.
2. Temporarily raise auth/rate-limit strictness.
3. Disable affected feature flags/routes.

## Recovery
1. Apply patch with peer review.
2. Replay stuck jobs from DLQ where safe.
3. Run audit-chain verification on impacted policies.
4. Verify metrics and alert noise return to baseline.

## Post-Incident
1. Publish timeline and root cause.
2. Add regression tests and detection alerts.
3. Rotate secrets and invalidate stale sessions if needed.
