# k6 Load Tests

Run with:

```bash
k6 run infra/k6/auth.js
k6 run infra/k6/vault-write.js
k6 run infra/k6/heartbeat.js
```

Supported env vars:

- `BASE_URL` (default `http://localhost:8080`)
- `AUTH_BEARER` bearer token for protected endpoints
- `DEVICE_ID` device id for replay headers
- `POLICY_ID` policy id for heartbeat scenario
- `AEAD_NONCE` and `AEAD_CIPHERTEXT` fixture envelope for encrypted endpoints

Threshold target:

- `p(99) < 200ms`
