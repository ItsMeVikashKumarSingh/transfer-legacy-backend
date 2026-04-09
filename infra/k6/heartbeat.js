import http from "k6/http";
import { check } from "k6";

export const options = {
  scenarios: {
    heartbeat: {
      executor: "constant-vus",
      vus: 1000,
      duration: "90s",
    },
  },
  thresholds: {
    http_req_duration: ["p(99)<200"],
  },
};

const BASE_URL = __ENV.BASE_URL || "http://localhost:8080";
const AUTH = __ENV.AUTH_BEARER || "";
const DEVICE_ID = __ENV.DEVICE_ID || "heartbeat-device";
const POLICY_ID = __ENV.POLICY_ID || "00000000-0000-0000-0000-000000000000";

export default function () {
  const payload = JSON.stringify({
    nonce: __ENV.AEAD_NONCE || "",
    ciphertext: __ENV.AEAD_CIPHERTEXT || "",
    policy_id: POLICY_ID,
  });
  const res = http.post(`${BASE_URL}/v1/inheritance/heartbeat`, payload, {
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${AUTH}`,
      "x-seq": `${__ITER + 1}`,
      "x-timestamp": `${Math.floor(Date.now() / 1000)}`,
      "x-device-id": DEVICE_ID,
      "x-idempotency-key": `hb-${__VU}-${__ITER}`,
    },
  });
  check(res, {
    "status is 200/400/401": (r) => r.status === 200 || r.status === 400 || r.status === 401,
  });
}
