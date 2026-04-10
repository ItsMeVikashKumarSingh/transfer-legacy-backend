# Instance Support Matrix

This project supports multiple Linux instance types through a tiered matrix.
The matrix defines test depth and release gates per architecture.

## Tier Definitions

| Tier | Scope | Release Gate |
|---|---|---|
| Tier 1 | Linux `amd64` and Linux `arm64` on mainstream VPS/cloud | Blocking |
| Tier 2 | Other Linux shapes that pass smoke and critical security checks | Non-blocking |
| Tier 3 | Experimental or unsupported platforms | Best effort |

## Minimum Runtime Resources

| Profile | vCPU | RAM | Storage | Typical Use |
|---|---:|---:|---:|---|
| `small` | 2 | 4 GB | 40 GB SSD | Dev and low traffic staging |
| `standard` | 4 | 8 GB | 80 GB SSD | Default production baseline |
| `high-memory` | 4 | 24 GB | 200 GB SSD | High queue volume and claim peaks |

## Architecture Requirements

- API and worker images must build for `linux/amd64` and `linux/arm64`.
- Tier 1 CI must run contract/security/integration gates on both architectures.
- Docker image scanning must include both architectures.

## Environment Separation

- `local`, `staging`, and `production` remain isolated environments.
- Each environment uses separate secrets, database projects, Valkey/Redis instances,
  and observability projects.
- Non-local environments load secrets from OpenBao KV only.

## Operational Boundaries

- Tier 1 incidents are treated as production defects and block release.
- Tier 2 failures do not block release but must be tracked and documented.
- Tier 3 has no release SLA and is not part of CI blocking criteria.
