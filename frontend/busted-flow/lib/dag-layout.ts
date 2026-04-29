import type { FlowNode, FlowEdge, Position } from "./flow-types"

export function layeredLayout(nodes: FlowNode[], edges: FlowEdge[]): Record<string, Position> {
  if (nodes.length === 0) return {}
  const adj: Record<string, string[]> = {}, inDeg: Record<string, number> = {}
  nodes.forEach(n => { adj[n.id] = []; inDeg[n.id] = 0 })
  edges.forEach(e => { adj[e.source].push(e.target); inDeg[e.target]++ })
  const layers = assignLayers(nodes, adj, inDeg)
  const ordered = minimizeCrossings(layers, edges)
  return assignPositions(ordered, 60, 140)
}

function assignLayers(nodes: FlowNode[], adj: Record<string, string[]>, inDeg: Record<string, number>): string[][] {
  const layers: string[][] = [], nodeLayer: Record<string, number> = {}, q: string[] = [], deg = { ...inDeg }
  Object.entries(deg).forEach(([id, d]) => { if (d === 0) q.push(id) })
  if (q.length === 0 && nodes.length > 0) q.push(nodes[0].id)
  while (q.length > 0) {
    const id = q.shift()!; const layer = nodeLayer[id] ?? 0
    while (layers.length <= layer) layers.push([])
    layers[layer].push(id)
    adj[id]?.forEach(nid => {
      const nl = layer + 1; nodeLayer[nid] = Math.max(nodeLayer[nid] ?? 0, nl)
      if (deg[nid]) { deg[nid]--; if (deg[nid] === 0) q.push(nid) }
    })
  }
  const layered = new Set(layers.flat()); const orphans = nodes.filter(n => !layered.has(n.id)).map(n => n.id)
  if (orphans.length > 0) layers.push(orphans)
  return layers
}

function minimizeCrossings(layers: string[][], edges: FlowEdge[]): string[][] {
  const ordered = layers.map(l => [...l]), edgeMap: Record<string, string[]> = {}
  edges.forEach(e => { (edgeMap[e.source] ??= []).push(e.target) })
  for (let pass = 0; pass < 4; pass++) {
    for (let i = 1; i < ordered.length; i++) {
      const prev = ordered[i - 1], curr = ordered[i]
      const bary = curr.map(id => {
        const pos = prev.map((pid, idx) => (edgeMap[pid]?.includes(id) ? idx : -1)).filter(x => x >= 0)
        return [id, pos.length ? pos.reduce((a, b) => a + b, 0) / pos.length : 0] as [string, number]
      })
      bary.sort((a, b) => a[1] - b[1])
      ordered[i] = bary.map(x => x[0])
    }
  }
  return ordered
}

function assignPositions(layers: string[][], nodeSpacing: number, layerSpacing: number): Record<string, Position> {
  const pos: Record<string, Position> = {}
  layers.forEach((layer, ly) => {
    const w = layer.length * (240 + nodeSpacing), sx = -w / 2
    layer.forEach((id, idx) => { pos[id] = { x: sx + idx * (240 + nodeSpacing), y: ly * layerSpacing } })
  })
  return pos
}
