#!/usr/bin/env bash
set -euo pipefail

repo="${1:-robo-txt/stealth-oxide}"
branch="${2:-main}"
root="$(git rev-parse --show-toplevel)"

gh auth status >/dev/null
gh api \
  --method PUT \
  -H "Accept: application/vnd.github+json" \
  -H "X-GitHub-Api-Version: 2022-11-28" \
  "repos/${repo}/branches/${branch}/protection" \
  --input "${root}/.github/branch-protection.json"

echo "Protected ${repo}:${branch}; Required CI must pass before changes can merge."
