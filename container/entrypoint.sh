#!/bin/sh
set -eu

# Refresh the per-user cache so fonts mounted by Kubernetes or Compose after
# image construction are visible to Chromium.
fc-cache -f >/dev/null
exec "$@"
