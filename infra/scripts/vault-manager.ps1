# Transfer Legacy - Unified Vault Manager
# This script handles secret synchronization and Vault lifecycle management.

param (
    [Parameter(Mandatory=$false)]
    [ValidateSet("init-test", "sync-file", "sync-env", "setup-approle")]
    [string]$Mode = "init-test",

    [Parameter(Mandatory=$false)]
    [string]$EnvFile = ".env.local",

    [Parameter(Mandatory=$false)]
    [string]$VaultPath = "secret/data/transfer-legacy/local",

    [Parameter(Mandatory=$false)]
    [string]$VaultAddr = "http://localhost:8200"
)

# --- Configuration & Auth ---
$VaultToken = $env:VAULT_TOKEN
if (-not $VaultToken) { $VaultToken = "root" }

function Invoke-Vault {
    param($Method, $Path, $Body)
    $Url = "$VaultAddr/v1/$Path"
    $Headers = @{ "X-Vault-Token" = $VaultToken }
    
    if ($Body) {
        $Json = $Body | ConvertTo-Json -Depth 10
        Invoke-RestMethod -Uri $Url -Method $Method -Headers $Headers -ContentType "application/json" -Body $Json
    } else {
        Invoke-RestMethod -Uri $Url -Method $Method -Headers $Headers
    }
}

# --- Core Logic ---

if ($Mode -eq "init-test") {
    Write-Host "Initializing Ephemeral Test Vault..." -ForegroundColor Cyan
    
    # 1. Enable KV Engine
    try {
        Invoke-Vault -Method "POST" -Path "sys/mounts/secret" -Body @{ type = "kv"; options = @{ version = "2" } }
    } catch { Write-Host "  (KV Engine already enabled)" -ForegroundColor Gray }

    # 2. Setup AppRole Auth
    try {
        Invoke-Vault -Method "POST" -Path "sys/auth/approle" -Body @{ type = "approle" }
    } catch { Write-Host "  (AppRole already enabled)" -ForegroundColor Gray }

    # 3. Create Policy
    $Policy = 'path "secret/data/transfer-legacy/*" { capabilities = ["read", "list"] }'
    Invoke-Vault -Method "PUT" -Path "sys/policies/acl/transfer-legacy-policy" -Body @{ policy = $Policy }

    # 4. Create AppRole
    Invoke-Vault -Method "POST" -Path "auth/approle/role/transfer-legacy-app" -Body @{
        token_policies = @("transfer-legacy-policy")
        token_ttl = "1h"
        token_max_ttl = "4h"
    }

    Write-Host "Vault Ready for Test Data." -ForegroundColor Green
}

if ($Mode -eq "sync-file") {
    Write-Host "Syncing Secrets from $EnvFile to Vault..." -ForegroundColor Cyan
    if (-not (Test-Path $EnvFile)) {
        Write-Error "File not found: $EnvFile"
        exit 1
    }

    $Secrets = @{}
    Get-Content $EnvFile | ForEach-Object {
        if ($_ -match "^([^#\s][^=]*)=(.*)$") {
            $Key = $Matches[1].Trim()
            $Val = $Matches[2].Trim()
            $Secrets[$Key] = $Val
        }
    }

    # --- SPECIAL: Force Mock Resend in Test Env ---
    if ($VaultAddr -match "8201") { 
         Write-Host "Forcing RESEND_API_KEY to mock mode (Test Env)..." -ForegroundColor Yellow
         $Secrets["RESEND_API_KEY"] = "re_mock_123"
    }

    $Payload = @{ data = $Secrets }
    Invoke-Vault -Method "POST" -Path $VaultPath -Body $Payload
    Write-Host "Successfully pushed $($Secrets.Count) keys to $VaultPath" -ForegroundColor Green
}

if ($Mode -eq "setup-approle") {
    Write-Host "Generating AppRole Credentials..." -ForegroundColor Cyan
    $RoleIdRes = Invoke-Vault -Method "GET" -Path "auth/approle/role/transfer-legacy-app/role-id"
    $SecretIdRes = Invoke-Vault -Method "POST" -Path "auth/approle/role/transfer-legacy-app/secret-id"

    $RoleId = $RoleIdRes.data.role_id
    $SecretId = $SecretIdRes.data.secret_id
    
    Write-Host "ROLE_ID=$RoleId"
    Write-Host "SECRET_ID=$SecretId"
}
