"use client"
import { useState } from "react"
import type { ServiceType } from "@/lib/flow-types"
import { Icon } from "./icons"

const ST: Record<ServiceType, { label: string; desc: string; cls: string; icon: string }> = {
  service:          { label: "Service",        desc: "Stateless handlers",       cls: "bg-node-action/15 text-node-action border-node-action/30", icon: "database" },
  "virtual-object": { label: "Virtual Object", desc: "Stateful keyed entity",   cls: "bg-node-logic/15 text-node-logic border-node-logic/30",   icon: "box" },
  workflow:         { label: "Workflow",        desc: "Multi-step durable flow", cls: "bg-node-trigger/15 text-node-trigger border-node-trigger/30", icon: "workflow" },
}

function Btn({ label, onClick, disabled, children }: { label: string; onClick?: () => void; disabled?: boolean; children: React.ReactNode }) {
  return (
    <button onClick={onClick} disabled={disabled} title={label} aria-label={label}
      className={["flex h-8 w-8 items-center justify-center rounded-md transition-all duration-100 text-muted-foreground hover:text-foreground hover:bg-secondary", disabled ? "opacity-40 pointer-events-none" : ""].join(" ")}>
      {children}
    </button>
  )
}

export function FlowToolbar({ workflowName, onWorkflowNameChange, serviceType, onServiceTypeChange, nodeCount, edgeCount, zoom, onZoomIn, onZoomOut, onFitView, isExecuting, onToggleExecution, onAutoLayout, onShowHelp }: {
  workflowName: string; onWorkflowNameChange: (n: string) => void
  serviceType: ServiceType; onServiceTypeChange: (t: ServiceType) => void
  nodeCount: number; edgeCount: number; zoom: number
  onZoomIn: () => void; onZoomOut: () => void; onFitView: () => void
  isExecuting: boolean; onToggleExecution: () => void; onAutoLayout: () => void; onShowHelp: () => void
}) {
  const [showPicker, setShowPicker] = useState(false)
  const cfg = ST[serviceType]

  return (
    <header className="flex h-12 items-center justify-between border-b border-border bg-card px-3 gap-2">
      {/* left */}
      <div className="flex items-center gap-3 min-w-0">
        <div className="flex h-7 w-7 shrink-0 items-center justify-center rounded-md bg-primary/10"><Icon name="shield" className="h-3.5 w-3.5 text-primary" /></div>
        <input type="text" value={workflowName} onChange={(e) => onWorkflowNameChange(e.target.value)} spellCheck={false}
          className="h-7 bg-transparent text-[14px] font-semibold text-foreground outline-none border-none w-auto min-w-[100px] max-w-[200px]" />

        {/* service type */}
        <div className="relative">
          <button onClick={() => setShowPicker(!showPicker)}
            className={["inline-flex items-center gap-1.5 rounded-md border px-2 py-1 text-[10px] font-medium transition-colors hover:brightness-110", cfg.cls].join(" ")}>
            <Icon name={cfg.icon} className="h-3 w-3" />{cfg.label}
            <svg className="h-3 w-3 opacity-50" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"><path d="m6 9 6 6 6-6" /></svg>
          </button>
          {showPicker && (
            <>
              <div className="fixed inset-0 z-40" onClick={() => setShowPicker(false)} />
              <div className="absolute left-0 top-full mt-1 z-50 w-[220px] rounded-lg border border-border bg-card shadow-xl shadow-black/20 p-1">
                {(Object.entries(ST) as [ServiceType, typeof cfg][]).map(([t, c]) => (
                  <button key={t} onClick={() => { onServiceTypeChange(t); setShowPicker(false) }}
                    className={["flex w-full items-center gap-2.5 rounded-md px-2.5 py-2 text-left transition-colors", serviceType === t ? "bg-secondary" : "hover:bg-secondary/60"].join(" ")}>
                    <div className={["flex h-6 w-6 shrink-0 items-center justify-center rounded-md border", c.cls].join(" ")}><Icon name={c.icon} className="h-3 w-3" /></div>
                    <div className="flex flex-col">
                      <span className="text-[11px] font-medium text-foreground">{c.label}</span>
                      <span className="text-[9px] text-muted-foreground">{c.desc}</span>
                    </div>
                    {serviceType === t && <div className="ml-auto h-1.5 w-1.5 rounded-full bg-primary" />}
                  </button>
                ))}
              </div>
            </>
          )}
        </div>

        <div className="hidden md:flex items-center gap-2 text-[11px] text-muted-foreground">
          <span className="rounded bg-secondary/60 px-1.5 py-0.5 font-mono">{nodeCount}{" steps"}</span>
          <span className="rounded bg-secondary/60 px-1.5 py-0.5 font-mono">{edgeCount}{" edges"}</span>
        </div>
      </div>

      {/* center: zoom + layout */}
      <div className="flex items-center gap-0.5 rounded-lg border border-border bg-background/50 px-1 py-0.5">
        <Btn label="Zoom Out" onClick={onZoomOut}><Icon name="zoom-out" className="h-4 w-4" /></Btn>
        <span className="min-w-[3rem] text-center text-[11px] font-mono text-muted-foreground">{Math.round(zoom * 100)}{"%"}</span>
        <Btn label="Zoom In" onClick={onZoomIn}><Icon name="zoom-in" className="h-4 w-4" /></Btn>
        <div className="mx-1 h-4 w-px bg-border" />
        <Btn label="Fit View" onClick={onFitView}><Icon name="maximize" className="h-4 w-4" /></Btn>
        <div className="mx-1 h-4 w-px bg-border" />
        <Btn label="Auto Layout DAG" onClick={onAutoLayout}><Icon name="layers" className="h-4 w-4" /></Btn>
      </div>

      {/* right */}
      <div className="flex items-center gap-1">
        <Btn label="Undo" disabled><Icon name="undo-arrow" className="h-4 w-4" /></Btn>
        <Btn label="Redo" disabled><Icon name="redo" className="h-4 w-4" /></Btn>
        <div className="mx-1 h-5 w-px bg-border" />
        <Btn label="Deploy"><Icon name="save" className="h-4 w-4" /></Btn>
        <Btn label="Journal"><Icon name="terminal" className="h-4 w-4" /></Btn>
        <Btn label="Settings"><Icon name="settings" className="h-4 w-4" /></Btn>
        <Btn label="Help (Press ?)" onClick={onShowHelp}>
          <span className="font-bold text-[14px] leading-none">?</span>
        </Btn>

        <button onClick={onToggleExecution}
          className={["ml-1 flex h-8 items-center gap-1.5 rounded-md px-3 text-[12px] font-medium transition-colors",
            isExecuting ? "bg-destructive/90 text-destructive-foreground hover:bg-destructive" : "bg-primary text-primary-foreground hover:bg-primary/90",
          ].join(" ")}>
          {isExecuting ? <><Icon name="pause" className="h-3.5 w-3.5" />Cancel</> : <><Icon name="play" className="h-3.5 w-3.5" />Invoke</>}
        </button>
      </div>
    </header>
  )
}
