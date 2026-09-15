#!/usr/bin/env bash
# Regenerate the Spec Kit copy of the constitution from the canonical file.
#
# `constitution.md` at the repository root is the single source of truth
# (constitution rule 7: one source, no spec duplication). Spec Kit reads a copy
# at `.specify/memory/constitution.md`, so that copy is GENERATED, never edited
# by hand. CI runs this script and fails if the result differs from what is
# committed, which is what keeps the two from drifting again.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
canonical="$root/constitution.md"
mirror="$root/.specify/memory/constitution.md"

if [ ! -f "$canonical" ]; then
  echo "missing canonical constitution at $canonical" >&2
  exit 1
fi

{
  echo "<!-- GENERATED FILE - do not edit."
  echo "     Source of truth: /constitution.md (constitution rule 7)."
  echo "     Regenerate with: scripts/sync-constitution.sh -->"
  echo
  cat "$canonical"
} > "$mirror"

echo "wrote $mirror from $canonical"
