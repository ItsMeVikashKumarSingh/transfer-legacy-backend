#!/usr/bin/env sh
set -eu

curl -fsS "http://127.0.0.1:8080/health" > /dev/null
