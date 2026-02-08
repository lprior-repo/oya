#!/bin/bash
# Quick corruption test runner - single test per run for debugging

REPORT="/home/lewis/src/oya/zjj_manual_test_results.log"

echo "=== MANUAL CORRUPTION TEST ===" >> "$REPORT"
echo "Test: $1" >> "$REPORT"
echo "Time: $(date)" >> "$REPORT"
echo "" >> "$REPORT"

eval "$2" >> "$REPORT" 2>&1
exit_code=$?

echo "" >> "$REPORT"
echo "Exit code: $exit_code" >> "$REPORT"
echo "=================================" >> "$REPORT"
echo "" >> "$REPORT"

echo "Exit code: $exit_code"
cat "$REPORT" | tail -20
