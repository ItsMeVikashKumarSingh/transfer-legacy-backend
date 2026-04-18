# Transfer Legacy - Docker Test Orchestrator
# This script runs the backend test suite in a clean containerized environment.

Write-Host "Starting Dockerized Test Suite..." -ForegroundColor Cyan

# Ensure we are in the project root
$ProjectRoot = Get-Item "$PSScriptRoot\..\.."
Set-Location $ProjectRoot.FullName

# --- Step 1: Start dependencies ---
Write-Host "Starting background services (DB, Redis, Vault)..." -ForegroundColor Gray
docker compose -f infra/docker-compose.test.yml up -d test-db test-redis test-vault

# --- Step 2: Initialize Vault & Sync Secrets ---
Start-Sleep -Seconds 2
$VaultAddr = "http://localhost:8201"
$VaultScript = "$PSScriptRoot/vault-manager.ps1"

Write-Host "Injecting Secrets into Vault..." -ForegroundColor Gray
& $VaultScript -Mode "init-test" -VaultAddr $VaultAddr
& $VaultScript -Mode "sync-file" -VaultAddr $VaultAddr -EnvFile "$ProjectRoot/.env.local"

# --- Step 3: Get AppRole Credentials ---
Write-Host "Fetching AppRole Credentials..." -ForegroundColor Gray
$Creds = & $VaultScript -Mode "setup-approle" -VaultAddr $VaultAddr
$RoleId = ($Creds | Select-String "ROLE_ID=(.*)").Matches.Groups[1].Value
$SecretId = ($Creds | Select-String "SECRET_ID=(.*)").Matches.Groups[1].Value

# --- Step 4: Run Tests ---
Write-Host "Executing Test Suite..." -ForegroundColor Cyan
# Run with RUST_LOG=info and --nocapture to see hashes and debug output
docker compose -f infra/docker-compose.test.yml run --build -e ROLE_ID=$RoleId -e SECRET_ID=$SecretId -e RUST_LOG=info test-runner bash -c "cargo test -p transfer-legacy-api --bin transfer-legacy-api -- --nocapture"

$TestResult = $LASTEXITCODE

Write-Host "Cleaning up test environment..." -ForegroundColor Gray
docker compose -f infra/docker-compose.test.yml down -v

if ($TestResult -eq 0) {
    Write-Host "SUCCESS: All tests passed in Docker!" -ForegroundColor Green
} else {
    Write-Host ("FAILURE: One or more tests failed in Docker (Exit Code: " + $TestResult + ").") -ForegroundColor Red
}

exit $TestResult
