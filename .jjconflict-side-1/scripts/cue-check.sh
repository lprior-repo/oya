#!/usr/bin/env bash
set -euo pipefail

if ! command -v cue >/dev/null 2>&1; then
	echo "cue not installed; skipping cue validation"
	exit 0
fi

shopt -s nullglob
schemas=(.beads/schemas/*.cue)
implementations=(.beads/implementation/*.cue)

if ((${#schemas[@]} == 0)) || ((${#implementations[@]} == 0)); then
	echo "no cue schemas or implementations; skipping cue validation"
	exit 0
fi

for implementation in "${implementations[@]}"; do
	cue vet "$implementation" "${schemas[@]}"
done

echo "cue validation passed"
