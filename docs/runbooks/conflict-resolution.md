---
title: "Conflict Resolution Runbook"
date: "2026-04-05"
status: "draft"
---

## Purpose
Describe the operational steps to resolve conflicts triggered during
release readiness (multiple claimants or contradictory evidence).

## Triggers
- Policy moves to `conflict_pending`
- `ops.conflict_records` entry created
- After hold expiry, policy moves to `manual_review`

## Inputs
- `ops.conflict_records`
- `ops.manual_reviews`
- Audit trail for the policy (`audit.events`)
- Claims, attachments, and attestations linked to the policy

## Resolution Steps
1. Verify policy status is `manual_review`.
2. Review conflict reason and details in `ops.conflict_records`.
3. Validate all attached evidence hashes against stored metadata.
4. Confirm claimant identity and attestation signatures offline.
5. Record a decision in `ops.manual_reviews` with rationale.
6. Apply the decision via ops API (dual-operator required).

## Outputs
- Policy status updated to `released` or `cancelled`
- Audit entry appended for the decision
- Manual review marked `resolved`

## Notes
- Never resolve without dual-operator approval.
- Do not edit or delete audit events.
