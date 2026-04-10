# Environment Templates

These templates are provider-neutral and support local, staging, and production
separation.

## Usage

1. Copy the template that matches the environment.
2. Fill real values for image tags and internal token.
3. Export variables before running Docker Compose.

Example:

```sh
set -a
. infra/environments/staging.env.template
. infra/profiles/standard.runtime.env
set +a
docker compose -f infra/docker-compose.yml -f infra/docker-compose.staging.yml up -d
```
