---
title: "Manual Review Operations Runbook"
date: "2026-04-08"
status: "draft"
---

## Scope
Operational workflow for `manual_review` policies and dual-operator decisions.

## Inputs
- `ops.manual_reviews`
- `ops.conflict_records`
- `audit.events` for the policy
- claims, attachments, attestations, release records

## Required Controls
1. Two distinct operators are required for any final decision.
2. Both signatures must verify on the same canonical decision payload.
3. All decisions must append an audit event.

## Review Procedure
1. Open review in `open` status.
2. Validate conflict reason and linked evidence.
3. Confirm claim and attestation consistency.
4. Choose decision: `released` or `cancelled`.
5. Submit dual signatures and decision notes.

## Verification
1. Ensure review status becomes `resolved`.
2. Ensure policy transitions to expected terminal state.
3. Confirm `manual_review_decision` event exists in audit chain.
4. Verify no orphaned unresolved conflicts remain.
