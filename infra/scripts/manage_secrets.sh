#!/usr/bin/env bash
set -euo pipefail

# Transfer Legacy - Secret Operations Tool
# This tool allows updating, rolling back, and inspecting secrets in OpenBao
# and automatically signals the backend to hot-reload.

BAO_PATH="secret/data/transfer-legacy/prod"
CONTAINER_NAME="transfer-legacy-backend-api-1" # Default name from docker-compose

show_help() {
    echo "Usage: ./manage_secrets.sh [COMMAND] [ARGS]"
    echo ""
    echo "Commands:"
    echo "  update <KEY> <VALUE>   Update a specific secret value and signal reload."
    echo "  rollback <VERSION>      Roll back to a specific KV version and signal reload."
    echo "  inspect                 Show current configuration hashes and metadata."
    echo "  reload                 Manually signal the backend to reload configuration."
}

signal_backend() {
    echo "--- Signaling backend to reload configuration ---"
    if docker ps --format '{{.Names}}' | grep -q "^${CONTAINER_NAME}$"; then
        docker kill -s HUP "$CONTAINER_NAME"
        echo "SIGHUP sent to $CONTAINER_NAME."
    else
        echo "Warning: Container $CONTAINER_NAME not found. Skipping signal."
        echo "If you are running in a different container, signal it manually via: docker kill -s HUP <id>"
    fi
}

if [[ $# -eq 0 ]]; then
    show_help
    exit 0
fi

case "$1" in
    update)
        if [[ $# -ne 3 ]]; then
            echo "Error: update requires KEY and VALUE"
            exit 1
        fi
        KEY="$2"
        VALUE="$3"
        echo "--- Updating $KEY in OpenBao ---"
        # We use patch to avoid overwriting other keys in the same path
        # Assuming KV-v2 is enabled
        bao kv patch "$BAO_PATH" "$KEY=$VALUE"
        signal_backend
        ;;
    
    rollback)
        if [[ $# -ne 2 ]]; then
            echo "Error: rollback requires VERSION number"
            exit 1
        fi
        VERSION="$2"
        echo "--- Rolling back $BAO_PATH to version $VERSION ---"
        bao kv rollback -version="$VERSION" "$BAO_PATH"
        signal_backend
        ;;

    inspect)
        echo "--- Current OpenBao Metadata ---"
        bao kv metadata get "$BAO_PATH"
        echo ""
        echo "--- Requesting Hashed Identities from Live Backend ---"
        # Optional: We could hit a health check endpoint if implemented
        echo "Check your Audit Email for detailed hashed identity logs after reloading."
        ;;

    reload)
        signal_backend
        ;;

    *)
        show_help
        exit 1
        ;;
esac
