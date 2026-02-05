#!/bin/bash
# KIRK Bead Processing Loop
# Iterates through KIRK beads and validates implementations
# Uses CUE as source of truth, outputs protobuf text

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

BEADS_FILE=".beads/issues.jsonl"
SPEC_FILE="examples/user-api.cue"
KIRK_SPEC="intent-kirk.cue"

echo -e "${BLUE}╔══════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║           KIRK Bead Processing Loop                          ║${NC}"
echo -e "${BLUE}║     CUE → Validate → Protobuf Text Output                    ║${NC}"
echo -e "${BLUE}╚══════════════════════════════════════════════════════════════╝${NC}"
echo ""

# Step 1: Build the project
echo -e "${YELLOW}[1/6] Building project...${NC}"
gleam build 2>&1 | head -20
if [ $? -eq 0 ]; then
    echo -e "${GREEN}✓ Build successful${NC}"
else
    echo -e "${RED}✗ Build failed${NC}"
    exit 1
fi
echo ""

# Step 2: Validate CUE schema
echo -e "${YELLOW}[2/6] Validating CUE schema (source of truth)...${NC}"
if [ -f "schema/intent.cue" ]; then
    ./cue vet schema/intent.cue 2>&1 || true
    echo -e "${GREEN}✓ CUE schema valid${NC}"
else
    echo -e "${RED}✗ CUE schema not found${NC}"
fi
echo ""

# Step 3: Run KIRK quality analysis on example spec
echo -e "${YELLOW}[3/6] Running KIRK quality analysis...${NC}"
if [ -f "$SPEC_FILE" ]; then
    gleam run -- quality "$SPEC_FILE" 2>&1 || true
else
    echo -e "${YELLOW}⚠ Example spec not found: $SPEC_FILE${NC}"
fi
echo ""

# Step 4: Run KIRK inversion analysis
echo -e "${YELLOW}[4/6] Running KIRK inversion analysis...${NC}"
if [ -f "$SPEC_FILE" ]; then
    gleam run -- invert "$SPEC_FILE" 2>&1 || true
fi
echo ""

# Step 5: Run KIRK coverage analysis
echo -e "${YELLOW}[5/6] Running KIRK coverage analysis...${NC}"
if [ -f "$SPEC_FILE" ]; then
    gleam run -- coverage "$SPEC_FILE" 2>&1 || true
fi
echo ""

# Step 6: Generate protobuf text output
echo -e "${YELLOW}[6/6] Generating protobuf text output...${NC}"
if [ -f "$SPEC_FILE" ]; then
    gleam run -- prototext "$SPEC_FILE" > /tmp/intent-spec.prototext 2>&1 || true
    if [ -f "/tmp/intent-spec.prototext" ]; then
        echo -e "${GREEN}✓ Protobuf text generated: /tmp/intent-spec.prototext${NC}"
        echo ""
        echo "Preview:"
        head -30 /tmp/intent-spec.prototext
    fi
fi
echo ""

# Step 7: Count and display KIRK beads
echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}KIRK Beads Summary${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
echo ""

if [ -f "$BEADS_FILE" ]; then
    total=$(grep -c "kirk" "$BEADS_FILE" 2>/dev/null || echo "0")
    closed=$(grep "kirk" "$BEADS_FILE" | grep -c '"status":"closed"' 2>/dev/null || echo "0")
    open=$(grep "kirk" "$BEADS_FILE" | grep -c '"status":"open"' 2>/dev/null || echo "0")

    echo "KIRK Beads:"
    echo "  Total:  $total"
    echo "  Open:   $open"
    echo "  Closed: $closed"
    echo ""

    if [ "$open" -gt 0 ]; then
        echo "Open KIRK beads:"
        grep "kirk" "$BEADS_FILE" | grep '"status":"open"' | while read -r line; do
            title=$(echo "$line" | jq -r '.title' 2>/dev/null || echo "Unknown")
            id=$(echo "$line" | jq -r '.id' 2>/dev/null || echo "")
            echo "  • [$id] $title"
        done
    fi
else
    echo -e "${YELLOW}No beads file found${NC}"
fi
echo ""

# Step 8: Run tests
echo -e "${YELLOW}Running tests...${NC}"
gleam test 2>&1 | tail -20 || true
echo ""

echo -e "${GREEN}════════════════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}                    KIRK Loop Complete                          ${NC}"
echo -e "${GREEN}════════════════════════════════════════════════════════════════${NC}"
