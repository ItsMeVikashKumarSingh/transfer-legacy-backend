# OpenBao Unseal SOP

## Preconditions
- OpenBao running locally on 127.0.0.1:8200
- Unseal keys stored offline in separate secure locations

## Steps
1. Set `BAO_ADDR=http://127.0.0.1:8200`
2. Run `bao operator unseal` and provide the required key shares
3. Verify status: `bao status`

## Notes
- Never store unseal keys on the server.
- Rotate keys after any suspected exposure.
