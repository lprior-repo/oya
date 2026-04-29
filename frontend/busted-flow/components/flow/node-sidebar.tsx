"use client"
import { useState } from "react"
import { nodeTemplates, categoryLabels, categoryOrder, type NodeCategory, type NodeTemplate } from "@/lib/flow-types"
import { Icon } from "./icons"

const DOT: Record<string, string> = { entry: "bg-node-trigger", durable: "bg-node-action", state: "bg-chart-5", flow: "bg-node-logic", timing: "bg-node-output", signal: "bg-primary" }
const ICO_BG: Record<string, string> = { entry: "bg-node-trigger/10 text-node-trigger border-node-trigger/20", durable: "bg-node-action/10 text-node-action border-node-action/20", state: "bg-chart-5/10 text-chart-5 border-chart-5/20", flow: "bg-node-logic/10 text-node-logic border-node-logic/20", timing: "bg-node-output/10 text-node-output border-node-output/20", signal: "bg-primary/10 text-primary border-primary/20" }

export function NodeSidebar({ onAddNode }: { onAddNode: (t: NodeTemplate) => void }) {
  const [search, setSearch] = useState("")
  const [collapsed, setCollapsed] = useState<Record<string, boolean>>({})

  const filtered = nodeTemplates.filter((t) => (t.label + t.description + t.type).toLowerCase().includes(search.toLowerCase()))
  const grouped: Record<string, NodeTemplate[]> = {}
  for (const cat of categoryOrder) grouped[cat] = filtered.filter((t) => t.category === cat)

  return (
    <aside className="flex h-full w-[264px] shrink-0 flex-col border-r border-border bg-card">
      {/* header */}
      <div className="flex items-center gap-2.5 px-4 py-3 border-b border-border">
        <div className="flex h-6 w-6 items-center justify-center rounded-md bg-primary/10"><Icon name="shield" className="h-3.5 w-3.5 text-primary" /></div>
        <div className="flex flex-col">
          <span className="text-[13px] font-semibold text-foreground leading-tight">Restate Blocks</span>
          <span className="text-[10px] text-muted-foreground leading-tight">Durable building blocks</span>
        </div>
      </div>

      {/* search */}
      <div className="px-3 py-2.5">
        <div className="relative">
          <Icon name="search" className="absolute left-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-muted-foreground pointer-events-none" />
          <input type="text" placeholder="Search blocks..." value={search} onChange={(e) => setSearch(e.target.value)}
            className="h-8 w-full rounded-md border border-border bg-background pl-8 pr-3 text-[12px] text-foreground placeholder:text-muted-foreground focus:border-primary/50 focus:outline-none focus:ring-1 focus:ring-primary/30 transition-colors" />
        </div>
      </div>

      {/* list */}
      <div className="flex-1 overflow-y-auto px-3 pb-4">
        {categoryOrder.map((cat) => {
          const items = grouped[cat]
          if (!items || items.length === 0) return null
          const col = collapsed[cat]
          return (
            <div key={cat} className="mb-3">
              <button onClick={() => setCollapsed((p) => ({ ...p, [cat]: !p[cat] }))} className="flex w-full items-center gap-2 px-1 py-2 group/cat">
                <div className={["h-1.5 w-1.5 rounded-full shrink-0", DOT[cat]].join(" ")} />
                <span className="text-[11px] font-medium uppercase tracking-wider text-muted-foreground group-hover/cat:text-foreground transition-colors">{categoryLabels[cat as NodeCategory]}</span>
                <span className="ml-auto text-[10px] text-muted-foreground/50">{items.length}</span>
                <svg className={["h-3 w-3 text-muted-foreground/40 transition-transform", col ? "-rotate-90" : ""].join(" ")} xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"><path d="m6 9 6 6 6-6" /></svg>
              </button>
              {!col && (
                <div className="flex flex-col gap-0.5">
                  {items.map((t) => (
                    <button key={t.type}
                      draggable onDragStart={(e) => { e.dataTransfer.setData("application/flownode", JSON.stringify(t)); e.dataTransfer.effectAllowed = "move" }}
                      onClick={() => onAddNode(t)}
                      className="group flex items-center gap-2.5 rounded-md px-2 py-2 text-left transition-all duration-100 hover:bg-secondary/80 active:scale-[0.98] cursor-grab active:cursor-grabbing">
                      <div className={["flex h-7 w-7 shrink-0 items-center justify-center rounded-md border transition-colors", ICO_BG[cat]].join(" ")}><Icon name={t.icon} className="h-3.5 w-3.5" /></div>
                      <div className="flex flex-col min-w-0">
                        <span className="text-[12px] font-medium text-foreground leading-tight truncate">{t.label}</span>
                        <span className="text-[10px] text-muted-foreground leading-tight truncate">{t.description}</span>
                      </div>
                    </button>
                  ))}
                </div>
              )}
            </div>
          )
        })}
        {filtered.length === 0 && (
          <div className="flex flex-col items-center justify-center py-12 text-center">
            <Icon name="search" className="h-8 w-8 text-muted-foreground/30 mb-3" />
            <p className="text-[12px] text-muted-foreground">No blocks found</p>
          </div>
        )}
      </div>
    </aside>
  )
}
