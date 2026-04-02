path "transit/sign/tl-signing" {
  capabilities = ["update"]
}

path "transit/verify/tl-signing" {
  capabilities = ["update"]
}

path "kv/data/*" {
  capabilities = ["read"]
}
