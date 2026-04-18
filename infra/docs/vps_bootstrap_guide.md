# 🚀 VPS Bootstrap & Deployment Guide
This guide will take you from a fresh VPS to a production-ready **Transfer Legacy** backend.

---

## Part 1: Initial Server Setup
When you first get your VPS (likely Ubuntu 22.04 or 24.04), run these commands to secure it and install the engine.

### 1. Update & Install Docker
```bash
sudo apt update && sudo apt upgrade -y
sudo apt install -y docker.io docker-compose
sudo usermod -aG docker $USER
# Log out and log back in for the group change to take effect!
```

### 2. Configure Firewall (UFW)
Only open what is absolutely necessary.
```bash
sudo ufw allow ssh          # Port 22
sudo ufw allow http         # Port 80 (for reverse proxy)
sudo ufw allow https        # Port 443
sudo ufw enable
```

---

## Part 2: The Secret Life Cycle
We don't use `.env` files on the server. We use **OpenBao (Vault)**.

### 1. Initialize Vault (First Time Only)
Run our helper script from your **Local PC**:
```powershell
.\infra\scripts\vault-manager.ps1 -Mode setup-vps -VaultAddr "http://YOUR_VPS_IP:8200"
```
*Wait, why port 8200? Our docker-compose will handle this safely.*

### 2. Push Your Production Secrets
Prepare a file named `.env.production` on your local PC (use the same keys as `.env.local`). Then run:
```powershell
.\infra\scripts\vault-manager.ps1 -Mode sync-file -EnvFile ".env.production" -VaultAddr "http://YOUR_VPS_IP:8200" -VaultPath "secret/data/transfer-legacy/production"
```

---

## Part 3: Continuous Deployment (The Flow)
Once your secrets are in the VPS Vault, the rest is automatic!

1. **GitHub Builds**: When you push to `main`, GitHub Actions builds a private "container" of your app.
2. **GitHub Deploys**: GitHub tells your VPS to pull that container and restart.
3. **App Connects**: Your app starts, talks to the internal Vault, gets its keys, and goes live.

---

## 🔒 Security Pro-Tips
1. **Never share your `.env.production`**: It only lives on your PC.
2. **Use SSH Keys**: Disable password login on your VPS eventually.
3. **Backup**: Remember to back up your Postgres volume (`/var/lib/docker/volumes/...`).

---

## Troubleshooting
- **Vault 403?** Check if your `VAULT_TOKEN` on the server expired.
- **App 500?** Check logs: `docker compose logs -f api`.
