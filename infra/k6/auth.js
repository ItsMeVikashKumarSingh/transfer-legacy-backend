import http from "k6/http";
import { check } from "k6";

export const options = {
  scenarios: {
    opaque_registration: {
      executor: "constant-vus",
      vus: 100,
      duration: "2m",
    },
  },
  thresholds: {
    http_req_duration: ["p(99)<200"],
  },
};

const BASE_URL = __ENV.BASE_URL || "http://localhost:8080";

export default function () {
  const payload = JSON.stringify({
    email: `load+${__VU}-${__ITER}@example.test`,
    request_id: `${__VU}-${__ITER}`,
  });
  const res = http.post(`${BASE_URL}/v1/auth/register/init`, payload, {
    headers: { "Content-Type": "application/json" },
  });
  check(res, {
    "status is 200/400": (r) => r.status === 200 || r.status === 400,
  });
}
