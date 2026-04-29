"use client"
import { useCallback, useEffect, useRef, useState } from "react"
import type { FlowNode, FlowEdge, NodeTemplate, Position, ServiceType, InvocationStatus } from "@/lib/flow-types"
import { NODE_WIDTH, NODE_HEIGHT } from "@/lib/flow-types"
import { layeredLayout } from "@/lib/dag-layout"
import { FlowNodeComponent } from "./flow-node"
import { FlowEdges } from "./flow-edges"
import { NodeSidebar } from "./node-sidebar"
import { FlowToolbar } from "./flow-toolbar"
import { NodeConfigPanel } from "./node-config-panel"
import { ShortcutsPanel } from "./shortcuts-panel"

const INIT_NODES: FlowNode[] = [
  { id: "1",  position: { x: 350, y: 40 },  data: { label: "HTTP Handler",    description: "POST /SignupWorkflow/{userId}/run", icon: "globe",      category: "entry",   configured: true, status: "completed", journalIndex: 0 } },
  { id: "2",  position: { x: 350, y: 170 }, data: { label: "Durable Step",    description: "Create user in database",          icon: "shield",     category: "durable", configured: true, status: "completed", durableStepName: "create-user", journalIndex: 1 } },
  { id: "3",  position: { x: 350, y: 300 }, data: { label: "If / Else",       description: "Check if user creation succeeded", icon: "git-branch", category: "flow",    configured: true, status: "completed", journalIndex: 2 } },
  { id: "4",  position: { x: 100, y: 440 }, data: { label: "Durable Step",    description: "Send verification email",          icon: "shield",     category: "durable", configured: true, status: "completed", durableStepName: "send-email", journalIndex: 3 } },
  { id: "5",  position: { x: 600, y: 440 }, data: { label: "Set State",       description: "Store signup status = failed",     icon: "upload",     category: "state",   configured: true, status: "completed", stateKey: "signupStatus", journalIndex: 3 } },
  { id: "6",  position: { x: 100, y: 580 }, data: { label: "Durable Promise", description: "Await email link click",           icon: "sparkles",   category: "signal",  configured: true, status: "suspended", promiseName: "email-link-clicked", journalIndex: 4 } },
  { id: "7",  position: { x: 100, y: 720 }, data: { label: "Durable Step",    description: "Validate secret matches",          icon: "shield",     category: "durable", configured: true, status: "pending",   durableStepName: "validate-secret", journalIndex: 5 } },
  { id: "8",  position: { x: 100, y: 860 }, data: { label: "Service Call",    description: "Activate user account",            icon: "arrow-right",category: "durable", configured: true, status: "pending",   durableStepName: "activate-user", targetService: "UserService", targetHandler: "activate", journalIndex: 6 } },
  { id: "s1", position: { x: 500, y: 630 }, data: { label: "Signal Handler",  description: "click() - resolves email-link-clicked", icon: "radio", category: "signal",  configured: true, status: "running",   promiseName: "email-link-clicked" } },
]

const INIT_EDGES: FlowEdge[] = [
  { id: "e1-2", source: "1", target: "2" },
  { id: "e2-3", source: "2", target: "3" },
  { id: "e3-4", source: "3", target: "4", label: "true" },
  { id: "e3-5", source: "3", target: "5", label: "false" },
  { id: "e4-6", source: "4", target: "6" },
  { id: "e6-7", source: "6", target: "7" },
  { id: "e7-8", source: "7", target: "8" },
  { id: "es-6", source: "s1", target: "6", animated: true },
]

let nid = 20

export function FlowCanvas() {
  const [nodes, setNodes] = useState<FlowNode[]>(INIT_NODES)
  const [edges, setEdges] = useState<FlowEdge[]>(INIT_EDGES)
  const [selId, setSelId] = useState<string | null>(null)
  const [selEdgeId, setSelEdgeId] = useState<string | null>(null)
  const [wfName, setWfName] = useState("SignupWorkflow")
  const [svcType, setSvcType] = useState<ServiceType>("workflow")
  const [executing, setExecuting] = useState(false)
  const [pan, setPan] = useState<Position>({ x: 0, y: 0 })
  const [zoom, setZoom] = useState(0.85)
  const [showShortcuts, setShowShortcuts] = useState(false)

  const ref = useRef<HTMLDivElement>(null)
  const panning = useRef(false)
  const panS = useRef<Position>({ x: 0, y: 0 })
  const panO = useRef<Position>({ x: 0, y: 0 })
  const dragId = useRef<string | null>(null)
  const dragS = useRef<Position>({ x: 0, y: 0 })
  const dragO = useRef<Position>({ x: 0, y: 0 })

  const [connFrom, setConnFrom] = useState<{ nodeId: string; handleType: "source" | "target" } | null>(null)
  const [tempTo, setTempTo] = useState<Position | null>(null)

  const selNode = nodes.find((n) => n.id === selId) ?? null

  const toCanvas = useCallback((cx: number, cy: number): Position => {
    const r = ref.current?.getBoundingClientRect()
    return r ? { x: (cx - r.left - pan.x) / zoom, y: (cy - r.top - pan.y) / zoom } : { x: 0, y: 0 }
  }, [pan, zoom])

  const onCanvasDown = useCallback((e: React.MouseEvent) => {
    if (e.button !== 0) return
    setSelId(null); setSelEdgeId(null); panning.current = true; panS.current = { x: e.clientX, y: e.clientY }; panO.current = { ...pan }
  }, [pan])

  const onEdgeClick = useCallback((id: string) => { setSelEdgeId(id); setSelId(null) }, [])

  const onNodeDown = useCallback((e: React.MouseEvent, id: string) => {
    if (e.button !== 0) return
    const n = nodes.find((n) => n.id === id); if (!n) return
    dragId.current = id; dragS.current = { x: e.clientX, y: e.clientY }; dragO.current = { ...n.position }
  }, [nodes])

  const onNodeClick = useCallback((id: string) => setSelId(id), [])

  const onHandleDown = useCallback((e: React.MouseEvent, id: string, h: "source" | "target") => {
    e.stopPropagation(); setConnFrom({ nodeId: id, handleType: h }); setTempTo(toCanvas(e.clientX, e.clientY))
  }, [toCanvas])

  const onMove = useCallback((e: React.MouseEvent) => {
    if (panning.current) { setPan({ x: panO.current.x + (e.clientX - panS.current.x), y: panO.current.y + (e.clientY - panS.current.y) }); return }
    if (dragId.current) {
      const dx = (e.clientX - dragS.current.x) / zoom, dy = (e.clientY - dragS.current.y) / zoom
      const snap = e.shiftKey ? 20 : 0 // hold shift for snap-to-grid
      const rawX = dragO.current.x + dx, rawY = dragO.current.y + dy
      const x = snap ? Math.round(rawX / snap) * snap : rawX
      const y = snap ? Math.round(rawY / snap) * snap : rawY
      setNodes((p) => p.map((n) => n.id === dragId.current ? { ...n, position: { x, y } } : n))
      return
    }
    if (connFrom) setTempTo(toCanvas(e.clientX, e.clientY))
  }, [zoom, connFrom, toCanvas])

  const onUp = useCallback((e: React.MouseEvent) => {
    panning.current = false; dragId.current = null
    if (connFrom && tempTo) {
      const t = (e.target as HTMLElement).closest("[data-node-id]")
      const tid = t?.getAttribute("data-node-id")
      if (tid && tid !== connFrom.nodeId) {
        const src = connFrom.handleType === "source" ? connFrom.nodeId : tid
        const tgt = connFrom.handleType === "source" ? tid : connFrom.nodeId
        if (!edges.some((e) => e.source === src && e.target === tgt)) setEdges((p) => [...p, { id: `e${src}-${tgt}`, source: src, target: tgt }])
      }
      setConnFrom(null); setTempTo(null)
    }
  }, [connFrom, tempTo, edges])

  // wheel zoom
  const zR = useRef(zoom); const pR = useRef(pan); zR.current = zoom; pR.current = pan
  useEffect(() => {
    const el = ref.current; if (!el) return
    const h = (e: WheelEvent) => {
      e.preventDefault()
      const r = el.getBoundingClientRect(), mx = e.clientX - r.left, my = e.clientY - r.top
      const d = e.deltaY > 0 ? 0.92 : 1.08, z = zR.current, p = pR.current
      const nz = Math.min(Math.max(z * d, 0.15), 3)
      setPan({ x: mx - ((mx - p.x) / z) * nz, y: my - ((my - p.y) / z) * nz }); setZoom(nz)
    }
    el.addEventListener("wheel", h, { passive: false })
    return () => el.removeEventListener("wheel", h)
  }, [])

  // delete edge/node with Del/Backspace, cancel connection with Esc
  useEffect(() => {
    const h = (e: KeyboardEvent) => {
      if (e.key === "Delete" || e.key === "Backspace") {
        if (selEdgeId) { setEdges(p => p.filter(e => e.id !== selEdgeId)); setSelEdgeId(null); e.preventDefault() }
        else if (selId) { delNode(selId); e.preventDefault() }
      }
      if (e.key === "Escape") {
        if (showShortcuts) { setShowShortcuts(false); e.preventDefault() }
        else if (connFrom) { setConnFrom(null); setTempTo(null); e.preventDefault() }
        else { setSelId(null); setSelEdgeId(null) }
      }
      if (e.key === "?") { setShowShortcuts(p => !p); e.preventDefault() }
    }
    window.addEventListener("keydown", h)
    return () => window.removeEventListener("keydown", h)
  }, [selEdgeId, selId, delNode, connFrom, showShortcuts])

  const zoomIn = useCallback(() => setZoom((z) => Math.min(z * 1.2, 3)), [])
  const zoomOut = useCallback(() => setZoom((z) => Math.max(z * 0.8, 0.15)), [])
  const fitView = useCallback(() => {
    if (!nodes.length) return; const r = ref.current?.getBoundingClientRect(); if (!r) return
    const xs = nodes.map((n) => n.position.x), ys = nodes.map((n) => n.position.y)
    const [mnx, mny] = [Math.min(...xs), Math.min(...ys)]
    const [mxx, mxy] = [Math.max(...xs.map((x) => x + NODE_WIDTH)), Math.max(...ys.map((y) => y + NODE_HEIGHT))]
    const pad = 80, nz = Math.min(Math.max(Math.min((r.width - pad * 2) / (mxx - mnx), (r.height - pad * 2) / (mxy - mny)), 0.15), 1.5)
    setPan({ x: r.width / 2 - ((mnx + mxx) / 2) * nz, y: r.height / 2 - ((mny + mxy) / 2) * nz }); setZoom(nz)
  }, [nodes])

  const autoLayout = useCallback(() => {
    const pos = layeredLayout(nodes, edges)
    setNodes(p => p.map(n => pos[n.id] ? { ...n, position: pos[n.id] } : n))
    setTimeout(() => fitView(), 50)
  }, [nodes, edges, fitView])

  const addNode = useCallback((t: NodeTemplate) => {
    const r = ref.current?.getBoundingClientRect()
    const cx = r ? (r.width / 2 - pan.x) / zoom : 400, cy = r ? (r.height / 2 - pan.y) / zoom : 300
    const id = String(nid++)
    setNodes((p) => [...p, { id, position: { x: cx - NODE_WIDTH / 2, y: cy - NODE_HEIGHT / 2 }, data: { label: t.label, description: t.description, icon: t.icon, category: t.category, configured: false } }])
  }, [pan, zoom])

  const onDragOver = useCallback((e: React.DragEvent) => { e.preventDefault(); e.dataTransfer.dropEffect = "move" }, [])
  const onDrop = useCallback((e: React.DragEvent) => {
    e.preventDefault(); const raw = e.dataTransfer.getData("application/flownode"); if (!raw) return
    const t: NodeTemplate = JSON.parse(raw); const pos = toCanvas(e.clientX, e.clientY); const id = String(nid++)
    setNodes((p) => [...p, { id, position: { x: pos.x - NODE_WIDTH / 2, y: pos.y - NODE_HEIGHT / 2 }, data: { label: t.label, description: t.description, icon: t.icon, category: t.category, configured: false } }])
  }, [toCanvas])

  const delNode = useCallback((id: string) => { setNodes((p) => p.filter((n) => n.id !== id)); setEdges((p) => p.filter((e) => e.source !== id && e.target !== id)); setSelId(null) }, [])
  const dupNode = useCallback((n: FlowNode) => { const id = String(nid++); setNodes((p) => [...p, { id, position: { x: n.position.x + 40, y: n.position.y + 40 }, data: { ...n.data, status: undefined } }]); setSelId(id) }, [])
  const updateNode = useCallback((id: string, d: Partial<FlowNode["data"]>) => { setNodes((p) => p.map((n) => n.id === id ? { ...n, data: { ...n.data, ...d } } : n)) }, [])

  const toggleExec = useCallback(() => {
    if (executing) { setExecuting(false); setNodes((p) => p.map((n) => ({ ...n, data: { ...n.data, status: undefined } }))); setEdges((p) => p.map((e) => ({ ...e, animated: false }))); return }
    setExecuting(true)
    const order = ["1", "2", "3", "4", "6"]; let step = 0
    const iv = setInterval(() => {
      if (step >= order.length) { clearInterval(iv); return }
      const cur = order[step], prev = step > 0 ? order[step - 1] : null
      setNodes((p) => p.map((n) => {
        if (n.id === cur) return { ...n, data: { ...n.data, status: "running" as InvocationStatus } }
        if (n.id === prev) return { ...n, data: { ...n.data, status: "completed" as InvocationStatus } }
        return n
      }))
      setEdges((p) => p.map((e) => ({ ...e, animated: e.target === cur })))
      step++
      if (step >= order.length) setTimeout(() => {
        setNodes((p) => p.map((n) => {
          if (n.id === "6") return { ...n, data: { ...n.data, status: "suspended" as InvocationStatus } }
          if (n.id === "4") return { ...n, data: { ...n.data, status: "completed" as InvocationStatus } }
          return n
        }))
        setEdges((p) => p.map((e) => ({ ...e, animated: e.source === "s1" })))
        setExecuting(false)
      }, 800)
    }, 800)
  }, [executing])

  const tempEdge = connFrom && tempTo ? (() => {
    const n = nodes.find((n) => n.id === connFrom.nodeId); if (!n) return null
    const from: Position = connFrom.handleType === "source" ? { x: n.position.x + NODE_WIDTH / 2, y: n.position.y + NODE_HEIGHT } : { x: n.position.x + NODE_WIDTH / 2, y: n.position.y }
    return { from, to: tempTo }
  })() : null

  const dg = 20

  return (
    <div className="flex h-screen w-full flex-col bg-background overflow-hidden">
      <FlowToolbar workflowName={wfName} onWorkflowNameChange={setWfName} serviceType={svcType} onServiceTypeChange={setSvcType}
        nodeCount={nodes.length} edgeCount={edges.length} zoom={zoom} onZoomIn={zoomIn} onZoomOut={zoomOut} onFitView={fitView}
        isExecuting={executing} onToggleExecution={toggleExec} onAutoLayout={autoLayout} onShowHelp={() => setShowShortcuts(true)} />

      <div className="flex flex-1 overflow-hidden">
        <NodeSidebar onAddNode={addNode} />

        {/* canvas */}
        <div ref={ref} className="relative flex-1 overflow-hidden cursor-grab active:cursor-grabbing"
          onMouseDown={onCanvasDown} onMouseMove={onMove} onMouseUp={onUp} onMouseLeave={onUp} onDragOver={onDragOver} onDrop={onDrop}>

          {/* dot grid */}
          <div className="absolute inset-0 pointer-events-none" style={{
            backgroundImage: `radial-gradient(circle, var(--border) 1px, transparent 1px)`,
            backgroundSize: `${dg * zoom}px ${dg * zoom}px`,
            backgroundPosition: `${pan.x % (dg * zoom)}px ${pan.y % (dg * zoom)}px`,
          }} />

          {/* transform layer */}
          <div className="absolute origin-top-left" style={{ transform: `translate(${pan.x}px, ${pan.y}px) scale(${zoom})`, willChange: "transform" }}>
            <FlowEdges edges={edges} nodes={nodes} selectedEdgeId={selEdgeId} tempEdge={tempEdge} onEdgeClick={onEdgeClick} />
            {nodes.map((n) => (
              <FlowNodeComponent key={n.id} node={n} selected={n.id === selId} scale={zoom}
                onMouseDown={onNodeDown} onClick={onNodeClick} onHandleMouseDown={onHandleDown} />
            ))}
          </div>

          {/* minimap */}
          <div className="absolute bottom-4 right-4 w-[160px] h-[110px] rounded-lg border border-border bg-card/90 backdrop-blur-sm overflow-hidden pointer-events-none">
            <svg viewBox={`${Math.min(...nodes.map(n => n.position.x), 0) - 40} ${Math.min(...nodes.map(n => n.position.y), 0) - 40} ${Math.max(...nodes.map(n => n.position.x + NODE_WIDTH), 800) - Math.min(...nodes.map(n => n.position.x), 0) + 80} ${Math.max(...nodes.map(n => n.position.y + NODE_HEIGHT), 600) - Math.min(...nodes.map(n => n.position.y), 0) + 80}`} className="w-full h-full">
              {edges.map((e) => { const s = nodes.find((n) => n.id === e.source), t = nodes.find((n) => n.id === e.target); return s && t ? <line key={e.id} x1={s.position.x + NODE_WIDTH / 2} y1={s.position.y + NODE_HEIGHT} x2={t.position.x + NODE_WIDTH / 2} y2={t.position.y} className={e.animated ? "stroke-node-action" : "stroke-border"} strokeWidth="3" /> : null })}
              {nodes.map((n) => <rect key={n.id} x={n.position.x} y={n.position.y} width={NODE_WIDTH} height={NODE_HEIGHT} rx="4"
                className={n.id === selId ? "fill-primary/40 stroke-primary" : n.data.status === "completed" ? "fill-node-trigger/20 stroke-node-trigger/40" : n.data.status === "running" ? "fill-node-action/20 stroke-node-action/40" : n.data.status === "suspended" ? "fill-node-output/20 stroke-node-output/40" : "fill-secondary stroke-border"} strokeWidth="2" />)}
            </svg>
          </div>

          {/* executing banner */}
          {executing && (
            <div className="absolute top-3 left-1/2 -translate-x-1/2 flex items-center gap-2 rounded-full border border-node-action/30 bg-card/95 backdrop-blur-sm px-4 py-1.5 shadow-lg">
              <div className="h-2 w-2 rounded-full bg-node-action animate-pulse" />
              <span className="text-[11px] font-medium text-foreground">Executing workflow...</span>
              <span className="text-[10px] font-mono text-muted-foreground">Durable execution active</span>
            </div>
          )}

          {/* edge selection indicator */}
          {selEdgeId && (
            <div className="absolute bottom-3 left-1/2 -translate-x-1/2 flex items-center gap-2 rounded-full border border-primary/30 bg-card/95 backdrop-blur-sm px-3 py-1.5 shadow-lg">
              <div className="h-2 w-2 rounded-full bg-primary" />
              <span className="text-[11px] font-medium text-foreground">Edge selected</span>
              <div className="h-3 w-px bg-border mx-1" />
              <span className="text-[10px] text-muted-foreground">Press Del to remove</span>
            </div>
          )}

          {/* connection helper */}
          {connFrom && (
            <div className="absolute top-3 left-1/2 -translate-x-1/2 flex items-center gap-2 rounded-full border border-primary/30 bg-card/95 backdrop-blur-sm px-3 py-1.5 shadow-lg animate-in fade-in slide-in-from-top-2 duration-200">
              <div className="h-2 w-2 rounded-full bg-primary animate-pulse" />
              <span className="text-[11px] font-medium text-foreground">Drawing connection...</span>
              <div className="h-3 w-px bg-border mx-1" />
              <span className="text-[10px] text-muted-foreground">Click another node or press Esc</span>
            </div>
          )}
        </div>

        {selNode && <NodeConfigPanel node={selNode} onClose={() => setSelId(null)} onDelete={delNode} onDuplicate={dupNode} onUpdateNode={updateNode} />}
      </div>

      <ShortcutsPanel show={showShortcuts} onClose={() => setShowShortcuts(false)} />
    </div>
  )
}
