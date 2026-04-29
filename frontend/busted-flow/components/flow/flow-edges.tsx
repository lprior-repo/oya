"use client"
import type { FlowNode, FlowEdge, Position } from "@/lib/flow-types"
import { NODE_WIDTH, NODE_HEIGHT } from "@/lib/flow-types"

// Pure math -- port directly to Rust
function srcPt(n: FlowNode): Position { return { x: n.position.x + NODE_WIDTH / 2, y: n.position.y + NODE_HEIGHT } }
function tgtPt(n: FlowNode): Position { return { x: n.position.x + NODE_WIDTH / 2, y: n.position.y } }

function smoothStep(f: Position, t: Position): string {
  const dx = t.x - f.x, midY = (f.y + t.y) / 2, R = 8
  if (Math.abs(dx) < 2) return `M ${f.x} ${f.y} L ${t.x} ${t.y}`
  const sx = dx > 0 ? 1 : -1, r = Math.min(R, Math.abs(dx) / 2, Math.abs(t.y - f.y) / 4)
  return `M ${f.x} ${f.y} L ${f.x} ${midY - r} Q ${f.x} ${midY} ${f.x + sx * r} ${midY} L ${t.x - sx * r} ${midY} Q ${t.x} ${midY} ${t.x} ${midY + r} L ${t.x} ${t.y}`
}

export function FlowEdges({ edges, nodes, selectedEdgeId, tempEdge, onEdgeClick }: {
  edges: FlowEdge[]; nodes: FlowNode[]; selectedEdgeId: string | null
  tempEdge: { from: Position; to: Position } | null
  onEdgeClick?: (id: string) => void
}) {
  const m = new Map(nodes.map((n) => [n.id, n]))

  return (
    <svg className="absolute inset-0 pointer-events-none overflow-visible" style={{ width: "100%", height: "100%", zIndex: 0 }}>
      <defs>
        <marker id="arrow" markerWidth="12" markerHeight="10" refX="11" refY="5" orient="auto" markerUnits="strokeWidth">
          <path d="M 0 0 L 12 5 L 0 10 L 3 5 z" className="fill-border stroke-border" strokeWidth="0.5" />
        </marker>
        <marker id="arrow-active" markerWidth="12" markerHeight="10" refX="11" refY="5" orient="auto" markerUnits="strokeWidth">
          <path d="M 0 0 L 12 5 L 0 10 L 3 5 z" className="fill-primary stroke-primary" strokeWidth="0.5" />
        </marker>
        <marker id="arrow-running" markerWidth="12" markerHeight="10" refX="11" refY="5" orient="auto" markerUnits="strokeWidth">
          <path d="M 0 0 L 12 5 L 0 10 L 3 5 z" className="fill-node-action stroke-node-action" strokeWidth="0.5" />
        </marker>
        <linearGradient id="edge-gradient" x1="0%" y1="0%" x2="100%" y2="0%">
          <stop offset="0%" stopColor="var(--primary)" stopOpacity="0.3" />
          <stop offset="100%" stopColor="var(--primary)" stopOpacity="1" />
        </linearGradient>
      </defs>

      {edges.map((edge) => {
        const s = m.get(edge.source), t = m.get(edge.target)
        if (!s || !t) return null
        const from = srcPt(s), to = tgtPt(t), path = smoothStep(from, to)
        const sel = edge.id === selectedEdgeId, anim = edge.animated
        return (
          <g key={edge.id}>
            <path d={path} fill="none" stroke="transparent" strokeWidth="16" className="pointer-events-auto cursor-pointer hover:stroke-primary/10 transition-all"
              onClick={() => onEdgeClick?.(edge.id)} />
            <path d={path} fill="none" strokeWidth={sel ? 3 : 2} strokeDasharray={anim ? "6 4" : undefined}
              markerEnd={sel ? "url(#arrow-active)" : anim ? "url(#arrow-running)" : "url(#arrow)"}
              className={["transition-all duration-150 pointer-events-none", anim ? "stroke-node-action animate-pulse" : sel ? "stroke-primary drop-shadow-[0_0_8px_rgba(var(--primary),0.5)]" : "stroke-border hover:stroke-primary/40"].join(" ")} />
            {edge.label && (
              <g>
                <rect x={(from.x + to.x) / 2 - 16} y={(from.y + to.y) / 2 - 8} width="32" height="16" rx="4" className="fill-card stroke-border" strokeWidth="1" />
                <text x={(from.x + to.x) / 2} y={(from.y + to.y) / 2 + 4} textAnchor="middle" className="fill-muted-foreground text-[9px] font-mono pointer-events-none">{edge.label}</text>
              </g>
            )}
          </g>
        )
      })}

      {tempEdge && (
        <>
          <path d={smoothStep(tempEdge.from, tempEdge.to)} fill="none" strokeWidth="8" strokeDasharray="6 4" className="stroke-primary/20 animate-pulse" />
          <path d={smoothStep(tempEdge.from, tempEdge.to)} fill="none" strokeWidth="2.5" strokeDasharray="6 4" className="stroke-primary animate-pulse" markerEnd="url(#arrow-active)" />
        </>
      )}
    </svg>
  )
}
