# Sentry Alert Rules

Create the following metric alerts in Sentry:

1. `server_decrypt_attempts_total > 0` for 1 minute => `critical`
2. `nonce_reuse_detected_total > 0` for 1 minute => `critical`
3. `sum(rate(aead_failures_total[1m])) > 5` for 2 minutes => `warning`
4. `max(heartbeat_worker_lag_seconds) > 3600` for 5 minutes => `warning`
5. `max(job_queue_depth) > 500` for 5 minutes => `warning`

Notification targets:
- PagerDuty (critical)
- Ops email (warning and critical)
