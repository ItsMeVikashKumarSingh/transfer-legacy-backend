# Git Rules — Transfer Legacy Backend

## Branching
- `main` is production.
- `dev` is integration.
- Feature branches: `feat/<short-name>`
- Fix branches: `fix/<short-name>`
- Security branches: `security/<short-name>`
- Docs/rules branches: `docs/<short-name>`

## Commit style
Use conventional commits:
- `feat(api): add heartbeat verification endpoint`
- `fix(vault): reject unsupported crypto_version`
- `security(auth): strip token fields from logs`
- `docs(rules): update crypto rules`

## Pull request rules
- No direct commits to `main`.
- PR must pass formatting, linting, tests, dependency audit, secret scan, and migration validation.
- Any crypto/auth/release change requires explicit security review.
- Any DB schema change requires migration plus rollback note.
- Any API change requires contract update.

## Secrets and history
- Never commit `.env` files.
- If a secret is committed, rotate first, then rewrite history.
- CI must run secret scanning on every PR.
