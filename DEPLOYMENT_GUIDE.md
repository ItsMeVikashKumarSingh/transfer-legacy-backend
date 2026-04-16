# Transfer Legacy - VPS Deployment Guide

This guide details the procedure for deploying the **Secret-Native** Transfer Legacy backend to a VPS using Docker and OpenBao.

## Prerequisites
- A VPS with **Docker** and **Docker Compose** installed.
- **bao** CLI installed on your local machine or the VPS for the initial setup.
- Domain `api.transferlegacy.com` pointed to your VPS IP.

---

## Phase 1: Initial Secret Seeding

Before logic can boot, OpenBao must be initialized and seeded with cryptographic primitives.

1. **Start the Infrastructure (Temporarily)**:
   ```bash
   docker compose up -d openbao
   ```

2. **Initialize OpenBao**:
   If this is a fresh install, initialize and unseal OpenBao:
   ```bash
   # Inside the VPS
   docker exec -it transfer-legacy-backend-openbao-1 bao operator init
   ```
   **IMPORTANT**: Save the recovery keys and root token securely. Unseal the vault using the keys.

3. **Run the Seeding Script**:
   The `infra/scripts/generate_and_seed.sh` script automates key generation and OpenBao configuration.
   ```bash
   chmod +x infra/scripts/generate_and_seed.sh
   ./infra/scripts/generate_and_seed.sh
   ```
   This will output your `ROLE_ID` and `SECRET_ID`.

---

## Phase 2: Deployment

1. **Configure Environment**:
   Create a `.env` file for Docker Compose:
   ```env
   ROLE_ID=your-role-id
   SECRET_ID=your-secret-id
   REDIS_PASSWORD=choose-a-strong-password
   ```

2. **Launch the Stack**:
   ```bash
   docker compose up -d --build
   ```

---

## Phase 3: Post-Deployment Operations

### Unsealing After Reboot
If the VPS reboots, the OpenBao container will start but will be **SEALED**. The backend will fail to boot until you manually unseal OpenBao using your recovery keys.

### Live Operations: Secret Management
The `infra/scripts/manage_secrets.sh` utility simplifies routine tasks:

- **Update a Single Secret**: (e.g., rotating the Brevo API key)
  ```bash
  ./infra/scripts/manage_secrets.sh update BREVO_API_KEY "new-key-value"
  ```
- **Rollback to Previous Version**: (if a mistake was made)
  ```bash
  ./infra/scripts/manage_secrets.sh rollback 5
  ```
- **Inspect Metadata**: See current version and last updated time.
  ```bash
  ./infra/scripts/manage_secrets.sh inspect
  ```

Every update or rollback automatically sends a `SIGHUP` to the backend and triggers a **Security Alert** email to the owner, showing hashed identity diffs of sensitive keys.

### Recovery & Rollback
If you mistakenly update a secret and need the old one back, **don't panic**. OpenBao KV-v2 preserves all previous versions.

1. **Identify the previous version**: Check your Audit Email for the version number before the mistake.
2. **Rollback via CLI**:
   ```bash
   # Restore version 4
   docker exec -it <openbao_container_id> bao kv rollback -version=4 secret/transfer-legacy/prod
   ```
3. **Rollback via UI**: Navigate to the `secret/transfer-legacy/prod` path in the OpenBao Web UI, click "Versions", and select "Rollback" on the desired version.
4. **Notify Backend**: After rolling back in Vault, send another `SIGHUP` to the backend to load the restored secrets.

### Security Reminders
- Your secrets (AEAD, HMAC, OPAQUE, JWT) **never** exist on the VPS disk in plaintext.
- The `.env` only contains the AppRole credentials required to fetch the actual secrets into memory.
- The root token used during seeding should be revoked or stored in a physical safe/hardware wallet.
- OPAQUE and AEAD keys should be treated as **immutable**. Rotating them requires complex data migration logic.
