#!/bin/bash
# Lists files exceeding MAX_LINES (default 300)
MAX_LINES=${1:-300}

for f in src/*.rs; do
	if [ -f "$f" ]; then
		lines=$(wc -l <"$f")
		if [ "$lines" -gt "$MAX_LINES" ]; then
			echo "$f: $lines lines"
		fi
	fi
done
