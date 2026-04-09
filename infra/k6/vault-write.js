import http from "k6/http";
import { check } from "k6";

export const options = {
  scenarios: {
    vault_write: {
      executor: "constant-arrival-rate",
      rate: 500,
      timeUnit: "1s",
      duration: "2m",
      preAllocatedVUs: 120,
      maxVUs: 300,
    },
  },
  thresholds: {
    http_req_duration: ["p(99)<200"],
  },
};

const BASE_URL = __ENV.BASE_URL || "http://localhost:8080";
const AUTH = __ENV.AUTH_BEARER || "";
const DEVICE_ID = __ENV.DEVICE_ID || "load-device";

export default function () {
  const payload = JSON.stringify({
    nonce: __ENV.AEAD_NONCE || "",
    ciphertext: __ENV.AEAD_CIPHERTEXT || "",
  });
  const res = http.post(`${BASE_URL}/v1/vault/items`, payload, {
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${AUTH}`,
      "x-seq": `${__ITER + 1}`,
      "x-timestamp": `${Math.floor(Date.now() / 1000)}`,
      "x-device-id": DEVICE_ID,
      "x-idempotency-key": `vault-${__VU}-${__ITER}`,
    },
  });
  check(res, {
    "status is 200/400/401": (r) => r.status === 200 || r.status === 400 || r.status === 401,
  });
}
