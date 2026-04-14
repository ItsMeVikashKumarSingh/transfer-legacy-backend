$ErrorActionPreference = 'Stop'

$compose = "docker compose -f infra/docker-compose.yml"

Write-Output "Ensuring db container is up..."
Invoke-Expression "$compose up -d db" | Out-Null

Write-Output "Applying migrations in order..."
$files = Get-ChildItem -Path "migrations" -Filter "*.sql" | Sort-Object Name
foreach ($f in $files) {
  Write-Output ("- " + $f.Name)
  Invoke-Expression "$compose exec -T db psql -U postgres -d transfer_legacy -v ON_ERROR_STOP=1 -f /migrations/$($f.Name)" | Out-Null
}

Write-Output "Migrations applied."

