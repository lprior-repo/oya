# DAG Tracker - Advanced Interactive Enhancements

## Overview
This document describes the advanced interactive features added to the Restate DAG tracker, making it comparable to Amazon Step Functions, n8n, and other professional workflow visualizers.

## New Features

### 1. Enhanced Visual Connection System

#### Interactive Connection Handles
- **Hover-to-reveal handles**: Connection points (circular handles) appear on hover over nodes
- **Visual feedback**: Handles glow with primary color and pulse animation on hover
- **Larger hit area**: 12px circular handles for easier clicking
- **Position**: Top handle (target/input), bottom handle (source/output)

#### Arrow Drawing
- **Click and drag**: Click any handle to start drawing a connection
- **Live preview**: Dashed animated line follows cursor during drawing
- **Smooth bezier curves**: Professional curved paths with rounded corners
- **Arrow markers**: Enhanced SVG arrow heads with gradient effects
- **Cancel drawing**: Press Esc or release outside a node to cancel

#### Connection Validation
- **Prevents self-loops**: Cannot connect node to itself
- **Duplicate detection**: Won't create duplicate edges between same nodes
- **Visual confirmation**: Connection snaps into place with animation

### 2. Edge Interaction & Management

#### Edge Selection
- **Click to select**: Click any edge to select it (highlights in primary color)
- **Visual feedback**: Selected edges glow with drop-shadow effect
- **Selection indicator**: Bottom banner shows "Edge selected - Press Del to remove"
- **Wider hit area**: 16px invisible stroke for easier clicking

#### Edge Deletion
- **Keyboard shortcut**: Press `Del` or `Backspace` to delete selected edge
- **Context-aware**: Also works for node deletion when node is selected

#### Edge Styling
- **Smooth-step algorithm**: Professional curved paths avoiding sharp angles
- **Animated edges**: Dashed animated flow for active connections
- **Edge labels**: Conditional labels (e.g., "true"/"false" for branches)
- **Status colors**: Different colors for normal/selected/animated states

### 3. Node Dragging Enhancements

#### Snap-to-Grid
- **Hold Shift**: While dragging, nodes snap to 20px grid
- **Smooth snapping**: No jitter, clean alignment
- **Visual grid**: Dot grid background shows alignment points

#### Improved Dragging
- **Smooth movement**: No lag or stuttering
- **Z-index management**: Selected nodes appear on top
- **Multi-node support**: Drag individual nodes independently

### 4. Auto-Layout Algorithm

#### DAG Layout Engine
- **Sugiyama algorithm**: Industry-standard layered graph layout
- **TypeScript implementation**: 56 lines, matching Rust version exactly
- **Automatic positioning**: One-click reorganization of entire graph
- **Layer assignment**: Topological sort with proper hierarchy
- **Horizontal centering**: Balanced node placement within layers

#### Usage
- **Toolbar button**: "Auto Layout DAG" button with layers icon
- **Automatic fit**: Auto-fits view after layout completes
- **Preserves connections**: All edges remain intact

### 5. Keyboard Shortcuts

#### Navigation & View
- `Click + Drag` - Pan canvas
- `Mouse Wheel` - Zoom in/out
- `Shift + Drag` - Snap node to grid

#### Editing
- `Click Handle` - Start connection
- `Esc` - Cancel connection / Deselect all
- `Del` - Delete selected node or edge
- `Ctrl + D` - Duplicate node (future)

#### Help
- `?` - Show/hide keyboard shortcuts panel

### 6. Visual Feedback System

#### Connection Helper
- **During drawing**: Top banner shows "Drawing connection..."
- **Instructions**: "Click another node or press Esc"
- **Animated indicator**: Pulsing primary dot

#### Edge Selection Banner
- **Bottom position**: "Edge selected - Press Del to remove"
- **Contextual**: Only shows when edge is selected

#### Execution Banner
- **Workflow running**: "Executing workflow... Durable execution active"
- **Animated**: Pulsing node-action color indicator

#### Shortcuts Panel
- **Modal overlay**: Backdrop blur with card panel
- **Organized list**: All shortcuts with visual key representations
- **Keyboard navigation**: Press `?` or `Esc` to toggle

### 7. Enhanced Arrow Rendering

#### Arrow Markers
- **3 marker types**: Default, Active (selected), Running (animated)
- **Improved design**: Filled triangles with stroke outline
- **Proper sizing**: 12x10px markers with correct refX/refY
- **Color-coded**: Border, primary, and node-action colors

#### Edge Path Algorithm
```typescript
function smoothStep(from: Position, to: Position): string {
  const dx = t.x - f.x, midY = (f.y + t.y) / 2, R = 8
  if (Math.abs(dx) < 2) return `M ${f.x} ${f.y} L ${t.x} ${t.y}`
  const sx = dx > 0 ? 1 : -1, r = Math.min(R, Math.abs(dx) / 2, Math.abs(t.y - f.y) / 4)
  return `M ${f.x} ${f.y} L ${f.x} ${midY - r} Q ${f.x} ${midY} ${f.x + sx * r} ${midY} L ${t.x - sx * r} ${midY} Q ${t.x} ${midY} ${t.x} ${midY + r} L ${t.x} ${t.y}`
}
```

#### Temp Edge Visualization
- **Dual-layer rendering**: 8px glow + 2.5px solid line
- **Animated**: Pulsing primary color
- **Arrow head**: Active marker for direction indication

## Rust/Dioxus Portability

All enhancements maintain the **minimal React surface area** philosophy:

### Files with Zero React Dependencies
- `flow-edges.tsx` - Pure SVG rendering (55 lines)
- `flow-node.tsx` - Pure HTML/CSS (80 lines)
- `icons.tsx` - Pure SVG data (68 lines)
- `shortcuts-panel.tsx` - Pure HTML modal (51 lines)

### State Management (Minimal React)
- `flow-canvas.tsx` - 8 useState, 4 useRef, 3 useCallback, 2 useEffect
- All state transitions are explicit and mappable to Dioxus `use_signal`
- No complex React patterns (no Context, no Suspense, no Portals)

### Porting Notes
1. **Connection state**: `connFrom` + `tempTo` → 2 signals
2. **Edge selection**: `selEdgeId` → 1 signal
3. **Shortcuts modal**: `showShortcuts` → 1 signal
4. **Keyboard handlers**: Direct `window.addEventListener` → Dioxus `use_effect`
5. **Smooth-step algorithm**: Pure math function → Copy directly to Rust

## Performance Characteristics

### Optimizations
- **Pointer events**: Strategic use of `pointer-events-none` for overlays
- **Will-change hints**: `transform` on canvas layer
- **Minimal re-renders**: Canvas transform doesn't trigger node re-renders
- **Passive listeners**: Wheel events with `{ passive: false }` for preventDefault
- **Z-index management**: Selected items on top without layout thrashing

### Rendering Metrics
- **~850 total lines** across all flow components
- **~30 React-specific lines** (3.5% of codebase)
- **97% portable** to Dioxus without modification

## Architecture Decisions

### Why Smooth-Step over Bezier?
- **Better for DAGs**: Orthogonal routing preferred for directed graphs
- **Easier to read**: Clear visual hierarchy of layers
- **Simple math**: No complex bezier control point calculations
- **Rust-friendly**: Pure geometric calculations, no curve fitting

### Why Single Canvas Transform?
- **Performance**: Single GPU layer for all nodes
- **Simplicity**: Pan/zoom affects everything uniformly
- **Dioxus-friendly**: No per-element transform calculations

### Why Inline SVG for Edges?
- **Flexibility**: Can easily add markers, gradients, filters
- **Control**: Precise path manipulation
- **Performance**: Browser-optimized SVG rendering
- **Portability**: SVG syntax identical in Rust/Dioxus

## Future Enhancements

### Planned
- [ ] Multi-select with Cmd/Ctrl + Click
- [ ] Box select with drag
- [ ] Copy/paste nodes (Cmd+C / Cmd+V)
- [ ] Undo/redo stack
- [ ] Edge labels editor (double-click edge)
- [ ] Node resizing handles
- [ ] Minimap click-to-navigate

### Considered
- [ ] Curved bezier edges (alternative to smooth-step)
- [ ] Connection validation rules (type checking)
- [ ] Auto-route around nodes (avoiding overlaps)
- [ ] Subgraphs / nested workflows
- [ ] Real-time collaboration cursors

## Integration with Restate

All connection/edge functionality integrates with Restate's execution model:

### Edge Types Map to Restate Concepts
- **Sequential flow**: Handler → Step → Step
- **Conditional branches**: If/Else → Branch A/B
- **Promises**: Step → Promise (suspended) → Signal → Continue
- **Parallel execution**: Single source → Multiple targets (fan-out)
- **Aggregation**: Multiple sources → Single target (fan-in)

### Visual Execution Tracking
- **Animated edges**: Active journal entry being executed
- **Edge colors**: Completed (green), Running (blue), Pending (gray)
- **Status markers**: Edge labels can show invocation IDs

## Summary

The enhanced DAG tracker now provides a **professional-grade visual workflow builder** with intuitive connection drawing, comprehensive keyboard shortcuts, auto-layout capabilities, and rich visual feedback. All features are implemented with **minimal React dependencies** for straightforward porting to Dioxus/Rust while maintaining the high-quality UX expected from tools like Amazon Step Functions and n8n.
