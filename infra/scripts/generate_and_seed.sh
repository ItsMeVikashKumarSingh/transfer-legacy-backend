#!/bin/bash
set -e

# Seeding script for Transfer Legacy Production Secrets
# Requirement: bao CLI installed and logged in as root/admin.

echo "--- Generating Cryptographic Primitives ---"
SECRETS=$(cargo run -p infra-tool --release)

# Parse output (very simple parser)
OPAQUE=$(echo "$SECRETS" | grep OPAQUE_SERVER_SETUP_B64 | cut -d'=' -f2)
AEAD=$(echo "$SECRETS" | grep SERVER_AEAD_KEY_B64 | cut -d'=' -f2)
HMAC=$(echo "$SECRETS" | grep SERVER_HMAC_SECRET | cut -d'=' -f2)
JWT=$(echo "$SECRETS" | grep JWT_SECRET | cut -d'=' -f2)

echo "--- Seeding OpenBao KV Store ---"
# Strictly check that we are not accidentally reading from local .env
unset DATABASE_URL REDIS_URL SUPABASE_URL SUPABASE_KEY BREVO_API_KEY

# Prompt for Owner Email if not set
if [ -z "$OWNER_EMAIL" ]; then
    read -p "Enter Owner Email for Security Alerts: " OWNER_EMAIL
fi

bao kv put secret/transfer-legacy/prod \
    opaque_server_setup_b64="$OPAQUE" \
    server_aead_key_b64="$AEAD" \
    server_hmac_secret="$HMAC" \
    jwt_secret="$JWT" \
    owner_email="$OWNER_EMAIL" \
    security_template_id="9" \
    database_url="postgres://user:pass@db-host:5432/db" \
    redis_url="redis://:pass@redis-host:6379" \
    supabase_url="https://xyz.supabase.co" \
    supabase_key="service-role-key" \
    backblaze_b2_app_key="your-b2-app-key" \
    brevo_api_key="your-brevo-key"

echo "--- Generating AppRole Credentials for Backend ---"
# Enable auth method if not enabled
bao auth enable approle || true

# Write policy
cat <<EOF > /tmp/tl-backend-policy.hcl
path "secret/data/transfer-legacy/prod" {
  capabilities = ["read"]
}
EOF
bao policy write transfer-legacy-backend /tmp/tl-backend-policy.hcl

# Create role
bao write auth/approle/role/transfer-legacy-backend \
    token_policies="transfer-legacy-backend" \
    token_ttl=1h \
    token_max_ttl=4h

# Fetch RoleID and SecretID
ROLE_ID=$(bao read -format=json auth/approle/role/transfer-legacy-backend/role-id | jq -r .data.role_id)
SECRET_ID=$(bao write -f -format=json auth/approle/role/transfer-legacy-backend/secret-id | jq -r .data.secret_id)

echo "***************************************************"
echo "PROD CONFIGURATION COMPLETE"
echo "***************************************************"
echo "ROLE_ID=$ROLE_ID"
echo "SECRET_ID=$SECRET_ID"
echo "***************************************************"
echo "Save these values! You will need them in your docker-compose.env"
