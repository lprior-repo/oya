#!/bin/bash
# Functional Rust: Convert all test .expect() to Result<(), Box<dyn std::error::Error>>

set -e

echo "Converting all tests to functional Result pattern..."

# Find all test functions and convert them
find src -name "*.rs" -type f -exec sed -i \
  -e 's/#\[test\]/#\[test\]\n    fn test_\(.*\)() -> Result<(), Box<dyn std::error::Error>> {/g' \
  {} \;

echo "Conversion complete. Manual review required for complex cases."
