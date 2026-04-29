"use client"
import type { FlowNode, InvocationStatus } from "@/lib/flow-types"
import { NODE_WIDTH } from "@/lib/flow-types"
import { Icon } from "./icons"

const BORDER: Record<string, string> = { entry: "border-node-trigger/40", durable: "border-node-action/40", state: "border-chart-5/40", flow: "border-node-logic/40", timing: "border-node-output/40", signal: "border-primary/40" }
const ICON_BG: Record<string, string> = { entry: "bg-node-trigger/15 text-node-trigger", durable: "bg-node-action/15 text-node-action", state: "bg-chart-5/15 text-chart-5", flow: "bg-node-logic/15 text-node-logic", timing: "bg-node-output/15 text-node-output", signal: "bg-primary/15 text-primary" }
const ACCENT: Record<string, string> = { entry: "bg-node-trigger", durable: "bg-node-action", state: "bg-chart-5", flow: "bg-node-logic", timing: "bg-node-output", signal: "bg-primary" }
const STATUS_CLS: Record<string, string> = { running: "bg-node-action/15 text-node-action border-node-action/30", suspended: "bg-node-output/15 text-node-output border-node-output/30", completed: "bg-node-trigger/15 text-node-trigger border-node-trigger/30", failed: "bg-destructive/15 text-destructive border-destructive/30", retrying: "bg-node-logic/15 text-node-logic border-node-logic/30" }
const STATUS_LABEL: Record<string, string> = { running: "Running", suspended: "Suspended", completed: "Done", failed: "Failed", retrying: "Retrying" }
const STATUS_ICON: Record<string, string> = { running: "loader", suspended: "pause", completed: "check-circle", failed: "alert-circle", retrying: "refresh" }
const STATUS_SPIN: Record<string, boolean> = { running: true, retrying: true }

export function FlowNodeComponent({ node, selected, onMouseDown, onClick, onHandleMouseDown }: {
  node: FlowNode; selected: boolean; scale: number
  onMouseDown: (e: React.MouseEvent, id: string) => void
  onClick: (id: string) => void
  onHandleMouseDown: (e: React.MouseEvent, id: string, h: "source" | "target") => void
}) {
  const { data } = node
  const cat = data.category

  return (
    <div
      data-node-id={node.id}
      className={[
        "absolute select-none group rounded-lg border bg-card transition-shadow duration-150 cursor-grab active:cursor-grabbing",
        BORDER[cat] || "border-border",
        selected ? "ring-2 ring-primary/60 border-primary/40 shadow-lg shadow-primary/10" : "hover:border-foreground/20 hover:shadow-md hover:shadow-black/10",
      ].join(" ")}
      style={{ left: node.position.x, top: node.position.y, width: NODE_WIDTH, zIndex: selected ? 10 : 1 }}
      onMouseDown={(e) => { e.stopPropagation(); onMouseDown(e, node.id) }}
      onClick={(e) => { e.stopPropagation(); onClick(node.id) }}
    >
      {/* accent bar */}
      <div className={["h-[2px] rounded-t-lg", ACCENT[cat] || "bg-border"].join(" ")} />

      {/* target handle */}
      <div className="absolute -top-[6px] left-1/2 -translate-x-1/2 h-3 w-3 rounded-full border-2 border-border bg-card hover:bg-primary hover:border-primary hover:scale-125 hover:shadow-lg hover:shadow-primary/30 transition-all duration-150 cursor-crosshair z-10 opacity-0 group-hover:opacity-100"
        onMouseDown={(e) => { e.stopPropagation(); onHandleMouseDown(e, node.id, "target") }}>
        <div className="absolute inset-0 rounded-full bg-primary/20 animate-pulse opacity-0 hover:opacity-100" />
      </div>

      {/* body */}
      <div className="flex items-center gap-3 px-3 py-2.5">
        <div className={["flex h-8 w-8 shrink-0 items-center justify-center rounded-md", ICON_BG[cat] || "bg-secondary text-foreground"].join(" ")}>
          <Icon name={data.icon} className="h-4 w-4" />
        </div>
        <div className="flex flex-col gap-0.5 min-w-0 flex-1">
          <span className="text-[13px] font-medium leading-tight text-foreground truncate">{data.label}</span>
          <span className="text-[10px] leading-tight text-muted-foreground truncate">
            {data.durableStepName ? <span className="font-mono">{"ctx.run(\""}{data.durableStepName}{"\")"}</span> : data.description}
          </span>
        </div>
        <div className="ml-auto shrink-0">
          {data.status && data.status !== "pending" && STATUS_CLS[data.status] ? (
            <span className={["inline-flex items-center gap-1 rounded-full border px-1.5 py-px text-[9px] font-medium leading-none", STATUS_CLS[data.status]].join(" ")}>
              <Icon name={STATUS_ICON[data.status]} className={["h-2.5 w-2.5", STATUS_SPIN[data.status] ? "animate-spin" : ""].join(" ")} />
              {STATUS_LABEL[data.status]}
            </span>
          ) : !data.status && data.configured ? (
            <div className="h-1.5 w-1.5 rounded-full bg-node-trigger" />
          ) : null}
        </div>
      </div>

      {/* journal row */}
      {(data.journalIndex !== undefined || (data.retryCount !== undefined && data.retryCount > 0)) && (
        <div className="flex items-center gap-2 px-3 pb-2 text-[9px] font-mono text-muted-foreground/70">
          {data.journalIndex !== undefined && <span className="rounded bg-secondary/60 px-1 py-px">{"journal #"}{data.journalIndex}</span>}
          {data.retryCount !== undefined && data.retryCount > 0 && <span className="rounded bg-destructive/10 text-destructive/70 px-1 py-px">{data.retryCount}{" retries"}</span>}
          {data.idempotencyKey && <span className="rounded bg-secondary/60 px-1 py-px truncate max-w-[80px]" title={data.idempotencyKey}>{"key: "}{data.idempotencyKey}</span>}
        </div>
      )}

      {/* source handle */}
      <div className="absolute -bottom-[6px] left-1/2 -translate-x-1/2 h-3 w-3 rounded-full border-2 border-border bg-card hover:bg-primary hover:border-primary hover:scale-125 hover:shadow-lg hover:shadow-primary/30 transition-all duration-150 cursor-crosshair z-10 opacity-0 group-hover:opacity-100"
        onMouseDown={(e) => { e.stopPropagation(); onHandleMouseDown(e, node.id, "source") }}>
        <div className="absolute inset-0 rounded-full bg-primary/20 animate-pulse opacity-0 hover:opacity-100" />
      </div>
    </div>
  )
}
