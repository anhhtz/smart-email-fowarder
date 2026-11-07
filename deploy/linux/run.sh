#!/usr/bin/env bash
set -e

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$DIR"

echo "========================================================"
echo "  Starting Smart Email Forwarder (Linux Console Mode)"
echo "========================================================"

if [ -f "./smart-email-forwarder" ]; then
    ./smart-email-forwarder --port 8080 --host 0.0.0.0
elif [ -f "./target/release/smart-email-forwarder" ]; then
    ./target/release/smart-email-forwarder --port 8080 --host 0.0.0.0
elif [ -f "./target/debug/smart-email-forwarder" ]; then
    ./target/debug/smart-email-forwarder --port 8080 --host 0.0.0.0
else
    echo "[ERROR] Executable smart-email-forwarder not found in $DIR"
    exit 1
fi
