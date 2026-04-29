"use client"
import { useState } from "react"
import type { FlowNode, NodeCategory, InvocationStatus } from "@/lib/flow-types"
import { Icon } from "./icons"

const CAT_BADGE: Record<string, string> = { entry: "bg-node-trigger/15 text-node-trigger border-node-trigger/25", durable: "bg-node-action/15 text-node-action border-node-action/25", state: "bg-chart-5/15 text-chart-5 border-chart-5/25", flow: "bg-node-logic/15 text-node-logic border-node-logic/25", timing: "bg-node-output/15 text-node-output border-node-output/25", signal: "bg-primary/15 text-primary border-primary/25" }
const STAT_BADGE: Record<string, string> = { pending: "bg-secondary text-muted-foreground border-border", running: "bg-node-action/15 text-node-action border-node-action/25", suspended: "bg-node-output/15 text-node-output border-node-output/25", completed: "bg-node-trigger/15 text-node-trigger border-node-trigger/25", failed: "bg-destructive/15 text-destructive border-destructive/25", retrying: "bg-node-logic/15 text-node-logic border-node-logic/25" }
const STAT_LABEL: Record<string, string> = { pending: "Pending", running: "Running", suspended: "Suspended", completed: "Completed", failed: "Failed", retrying: "Retrying" }
const STAT_ICON: Record<string, string> = { running: "loader", suspended: "pause", completed: "check-circle", failed: "alert-circle", retrying: "refresh" }
const STAT_SPIN: Record<string, boolean> = { running: true, retrying: true }

const inputCls = "h-8 rounded-md border border-border bg-background px-3 text-[12px] text-foreground font-mono focus:border-primary/50 focus:outline-none focus:ring-1 focus:ring-primary/30 transition-colors"

export function NodeConfigPanel({ node, onClose, onDelete, onDuplicate, onUpdateNode }: {
  node: FlowNode | null; onClose: () => void; onDelete: (id: string) => void
  onDuplicate: (n: FlowNode) => void; onUpdateNode: (id: string, d: Partial<FlowNode["data"]>) => void
}) {
  const [tab, setTab] = useState<"config" | "execution">("config")
  if (!node) return null

  const { data } = node
  const cat = data.category

  return (
    <aside className="flex h-full w-[320px] shrink-0 flex-col border-l border-border bg-card animate-in slide-in-from-right-2 duration-200">
      {/* header */}
      <div className="flex items-center justify-between px-4 py-3 border-b border-border">
        <div className="flex items-center gap-2.5 min-w-0">
          <div className={["flex h-7 w-7 shrink-0 items-center justify-center rounded-md border", CAT_BADGE[cat]].join(" ")}><Icon name={data.icon} className="h-3.5 w-3.5" /></div>
          <div className="min-w-0">
            <h3 className="text-[13px] font-semibold text-foreground truncate">{data.label}</h3>
            <p className="text-[10px] text-muted-foreground truncate">{data.description}</p>
          </div>
        </div>
        <button onClick={onClose} className="flex h-6 w-6 shrink-0 items-center justify-center rounded-md text-muted-foreground hover:text-foreground hover:bg-secondary transition-colors"><Icon name="x" className="h-3.5 w-3.5" /></button>
      </div>

      {/* tabs */}
      <div className="flex border-b border-border">
        {(["config", "execution"] as const).map((t) => (
          <button key={t} onClick={() => setTab(t)}
            className={["flex-1 py-2 text-[11px] font-medium capitalize transition-colors border-b-2", tab === t ? "text-foreground border-primary" : "text-muted-foreground border-transparent hover:text-foreground"].join(" ")}>
            {t === "config" ? "Configuration" : "Execution"}
          </button>
        ))}
      </div>

      {/* content */}
      <div className="flex-1 overflow-y-auto">
        {tab === "config" ? (
          <div className="flex flex-col gap-4 p-4">
            <div className="flex items-center gap-2 flex-wrap">
              <span className={["inline-flex items-center rounded-md border px-2 py-0.5 text-[10px] font-medium capitalize", CAT_BADGE[cat]].join(" ")}>{cat}</span>
              <span className="text-[10px] text-muted-foreground font-mono">{"ID: "}{node.id}</span>
            </div>

            {/* step name - for durable steps */}
            {cat === "durable" && (
              <div className="flex flex-col gap-1.5">
                <label className="text-[11px] font-medium text-muted-foreground uppercase tracking-wide">Step Name</label>
                <input type="text" defaultValue={data.durableStepName || data.label} onChange={(e) => onUpdateNode(node.id, { durableStepName: e.target.value })} className={inputCls} placeholder='e.g. "create-user"' />
                <span className="text-[10px] text-muted-foreground">{"ctx.run(\"name\", () => ...)"}</span>
              </div>
            )}

            {/* entry point configs */}
            {cat === "entry" && (
              <>
                {data.icon === "clock" && (
                  <div className="flex flex-col gap-1.5">
                    <label className="text-[11px] font-medium text-muted-foreground uppercase tracking-wide">Cron Expression</label>
                    <input type="text" defaultValue={data.cronExpression || ""} onChange={(e) => onUpdateNode(node.id, { cronExpression: e.target.value })} className={inputCls} placeholder='"0 */5 * * *" (every 5 min)' />
                  </div>
                )}
                {data.icon === "kafka" && (
                  <div className="flex flex-col gap-1.5">
                    <label className="text-[11px] font-medium text-muted-foreground uppercase tracking-wide">Kafka Topic</label>
                    <input type="text" defaultValue={data.topic || ""} onChange={(e) => onUpdateNode(node.id, { topic: e.target.value })} className={inputCls} placeholder="orders-topic" />
                  </div>
                )}
                {data.icon === "play-circle" && (
                  <div className="flex flex-col gap-1.5">
                    <label className="text-[11px] font-medium text-muted-foreground uppercase tracking-wide">Workflow Key</label>
                    <input type="text" defaultValue={data.workflowKey || ""} onChange={(e) => onUpdateNode(node.id, { workflowKey: e.target.value })} className={inputCls} placeholder="user-123" />
                  </div>
                )}
              </>
            )}

            {/* durable step configs */}
            {cat === "durable" && (
              <>
                <div className="flex flex-col gap-1.5">
                  <label className="text-[11px] font-medium text-muted-foreground uppercase tracking-wide">Target Service</label>
                  <input type="text" defaultValue={data.targetService || ""} onChange={(e) => onUpdateNode(node.id, { targetService: e.target.value })} className={inputCls} placeholder="PaymentService" />
                  <span className="text-[10px] text-muted-foreground font-mono">{"ctx.serviceClient<T>(\"name\")"}</span>
                </div>
                <div className="flex flex-col gap-1.5">
                  <label className="text-[11px] font-medium text-muted-foreground uppercase tracking-wide">Handler / Method</label>
                  <input type="text" defaultValue={data.targetHandler || ""} onChange={(e) => onUpdateNode(node.id, { targetHandler: e.target.value })} className={inputCls} placeholder="processPayment" />
                  <span className="text-[10px] text-muted-foreground font-mono">{".processPayment(req)"}</span>
                </div>
                {data.icon === "send" && (
                  <div className="rounded-lg border border-dashed border-node-action/30 bg-node-action/5 p-2">
                    <p className="text-[10px] text-muted-foreground">{"Fire-and-forget: ctx.objectSendClient<T>(key).method(req)"}</p>
                  </div>
                )}
                {data.icon === "clock-send" && (
                  <div className="flex flex-col gap-1.5">
                    <label className="text-[11px] font-medium text-muted-foreground uppercase tracking-wide">Delay Duration</label>
                    <input type="text" defaultValue={data.sleepDuration || ""} onChange={(e) => onUpdateNode(node.id, { sleepDuration: e.target.value })} className={inputCls} placeholder='"1h", "30m"' />
                    <span className="text-[10px] text-muted-foreground font-mono">{"ctx.objectSendClient(key, { delay: ... })"}</span>
                  </div>
                )}
              </>
            )}

            {/* state configs */}
            {cat === "state" && (
              <>
                <div className="flex flex-col gap-1.5">
                  <label className="text-[11px] font-medium text-muted-foreground uppercase tracking-wide">State Key</label>
                  <input type="text" defaultValue={data.stateKey || ""} onChange={(e) => onUpdateNode(node.id, { stateKey: e.target.value })} className={inputCls} placeholder='"cart", "profile"' />
                </div>
                <div className="rounded-lg border border-dashed border-chart-5/30 bg-chart-5/5 p-2">
                  <p className="text-[10px] text-muted-foreground font-mono leading-relaxed">
                    {data.icon === "download" && 'await ctx.get<T>("key")'}
                    {data.icon === "upload" && 'ctx.set("key", value)'}
                    {data.icon === "eraser" && 'ctx.clear("key") | clearAll()'}
                  </p>
                </div>
              </>
            )}

            {/* flow control configs */}
            {cat === "flow" && (
              <>
                {data.icon === "git-branch" && (
                  <div className="flex flex-col gap-1.5">
                    <label className="text-[11px] font-medium text-muted-foreground uppercase tracking-wide">Condition Expression</label>
                    <textarea defaultValue={data.conditionExpression || ""} onChange={(e) => onUpdateNode(node.id, { conditionExpression: e.target.value })} rows={2} className="rounded-md border border-border bg-background px-3 py-2 text-[11px] text-foreground font-mono focus:border-primary/50 focus:outline-none focus:ring-1 focus:ring-primary/30 transition-colors resize-none" placeholder="user.verified === true" />
                  </div>
                )}
                {data.icon === "repeat" && (
                  <div className="flex flex-col gap-1.5">
                    <label className="text-[11px] font-medium text-muted-foreground uppercase tracking-wide">Loop Iterator</label>
                    <input type="text" defaultValue={data.loopIterator || ""} onChange={(e) => onUpdateNode(node.id, { loopIterator: e.target.value })} className={inputCls} placeholder="items, userIds, tasks" />
                    <span className="text-[10px] text-muted-foreground font-mono">{"for (const item of items) { ... }"}</span>
                  </div>
                )}
                {data.icon === "undo" && (
                  <div className="flex flex-col gap-1.5">
                    <label className="text-[11px] font-medium text-muted-foreground uppercase tracking-wide">Compensation Handler</label>
                    <input type="text" defaultValue={data.compensationHandler || ""} onChange={(e) => onUpdateNode(node.id, { compensationHandler: e.target.value })} className={inputCls} placeholder="refundPayment" />
                    <span className="text-[10px] text-muted-foreground">Saga rollback logic</span>
                  </div>
                )}
              </>
            )}

            {/* timing configs */}
            {cat === "timing" && (
              <>
                {data.icon === "timer" && (
                  <div className="flex flex-col gap-1.5">
                    <label className="text-[11px] font-medium text-muted-foreground uppercase tracking-wide">Sleep Duration</label>
                    <input type="text" defaultValue={data.sleepDuration || ""} onChange={(e) => onUpdateNode(node.id, { sleepDuration: e.target.value })} className={inputCls} placeholder='"5m", "1h", "30s"' />
                    <span className="text-[10px] text-muted-foreground font-mono">{"await ctx.sleep(duration)"}</span>
                  </div>
                )}
                {data.icon === "alarm" && (
                  <div className="flex flex-col gap-1.5">
                    <label className="text-[11px] font-medium text-muted-foreground uppercase tracking-wide">Timeout (ms)</label>
                    <input type="number" defaultValue={data.timeoutMs || 30000} onChange={(e) => onUpdateNode(node.id, { timeoutMs: parseInt(e.target.value) })} className={inputCls} placeholder="30000" />
                    <span className="text-[10px] text-muted-foreground font-mono">{"promise.orTimeout(ms)"}</span>
                  </div>
                )}
              </>
            )}

            {/* signal/promise configs */}
            {cat === "signal" && (
              <>
                {(data.icon === "sparkles" || data.icon === "bell") && (
                  <div className="flex flex-col gap-1.5">
                    <label className="text-[11px] font-medium text-muted-foreground uppercase tracking-wide">Promise Name</label>
                    <input type="text" defaultValue={data.promiseName || ""} onChange={(e) => onUpdateNode(node.id, { promiseName: e.target.value })} className={inputCls} placeholder='"payment-completed"' />
                    <span className="text-[10px] text-muted-foreground font-mono">
                      {data.icon === "sparkles" && 'await ctx.promise<T>("name")'}
                      {data.icon === "bell" && 'const { id, promise } = ctx.awakeable<T>()'}
                    </span>
                  </div>
                )}
                {data.icon === "check-circle" && (
                  <div className="flex flex-col gap-1.5">
                    <label className="text-[11px] font-medium text-muted-foreground uppercase tracking-wide">Promise to Resolve</label>
                    <input type="text" defaultValue={data.promiseName || ""} onChange={(e) => onUpdateNode(node.id, { promiseName: e.target.value })} className={inputCls} placeholder='"payment-completed"' />
                    <span className="text-[10px] text-muted-foreground font-mono">{'ctx.promiseManager().resolve("name", val)'}</span>
                  </div>
                )}
                <div className="rounded-lg border border-dashed border-primary/30 bg-primary/5 p-2">
                  <p className="text-[10px] text-muted-foreground leading-relaxed">Durable promises suspend execution until resolved externally via HTTP or SDK.</p>
                </div>
              </>
            )}

            <div className="h-px bg-border" />

            {/* retry config */}
            <div className="flex flex-col gap-3">
              <label className="text-[11px] font-medium text-muted-foreground uppercase tracking-wide">Retry Policy</label>
              <div className="grid grid-cols-2 gap-2">
                <div className="flex flex-col gap-1">
                  <span className="text-[10px] text-muted-foreground">Max Retries</span>
                  <input type="number" defaultValue={3} className="h-7 rounded-md border border-border bg-background px-2 text-[11px] text-foreground font-mono focus:border-primary/50 focus:outline-none focus:ring-1 focus:ring-primary/30 transition-colors" />
                </div>
                <div className="flex flex-col gap-1">
                  <span className="text-[10px] text-muted-foreground">Backoff (ms)</span>
                  <input type="number" defaultValue={1000} className="h-7 rounded-md border border-border bg-background px-2 text-[11px] text-foreground font-mono focus:border-primary/50 focus:outline-none focus:ring-1 focus:ring-primary/30 transition-colors" />
                </div>
              </div>
            </div>

            {/* idempotency key */}
            <div className="flex flex-col gap-1.5">
              <label className="text-[11px] font-medium text-muted-foreground uppercase tracking-wide">Idempotency Key</label>
              <input type="text" defaultValue={data.idempotencyKey || ""} onChange={(e) => onUpdateNode(node.id, { idempotencyKey: e.target.value })} className={inputCls} placeholder="ctx.rand.uuidv4()" />
              <span className="text-[10px] text-muted-foreground">Auto-generated if blank. Ensures exactly-once execution.</span>
            </div>
          </div>
        ) : (
          <div className="flex flex-col gap-4 p-4">
            <div className="flex flex-col gap-2">
              <label className="text-[11px] font-medium text-muted-foreground uppercase tracking-wide">Invocation Status</label>
              {data.status ? (
                <div className={["inline-flex self-start items-center gap-1.5 rounded-md border px-2.5 py-1 text-[11px] font-medium", STAT_BADGE[data.status] || ""].join(" ")}>
                  {STAT_ICON[data.status] && <Icon name={STAT_ICON[data.status]} className={["h-3 w-3", STAT_SPIN[data.status] ? "animate-spin" : ""].join(" ")} />}
                  {STAT_LABEL[data.status] || data.status}
                </div>
              ) : <span className="text-[11px] text-muted-foreground">Not yet executed</span>}
            </div>

            {data.journalIndex !== undefined && (
              <div className="flex flex-col gap-1">
                <label className="text-[11px] font-medium text-muted-foreground uppercase tracking-wide">Journal Entry</label>
                <div className="flex items-center gap-2">
                  <span className="rounded bg-secondary px-2 py-0.5 text-[11px] font-mono text-foreground">{"#"}{data.journalIndex}</span>
                  <span className="text-[10px] text-muted-foreground">Position in durable execution log</span>
                </div>
              </div>
            )}

            {data.retryCount !== undefined && data.retryCount > 0 && (
              <div className="flex flex-col gap-1">
                <label className="text-[11px] font-medium text-muted-foreground uppercase tracking-wide">Retry Attempts</label>
                <div className="flex items-center gap-2">
                  <span className="rounded bg-destructive/10 px-2 py-0.5 text-[11px] font-mono text-destructive">{data.retryCount}</span>
                  <span className="text-[10px] text-muted-foreground">Times retried before success/failure</span>
                </div>
              </div>
            )}

            <div className="h-px bg-border" />
            <div className="rounded-lg border border-dashed border-border bg-secondary/20 p-3">
              <p className="text-[11px] text-muted-foreground leading-relaxed">Restate persists each step in a durable journal. On failure, execution replays from the journal, skipping already-completed steps. This ensures exactly-once semantics.</p>
            </div>
          </div>
        )}
      </div>

      {/* footer */}
      <div className="flex items-center gap-2 px-4 py-3 border-t border-border">
        <button onClick={() => onDuplicate(node)} className="flex h-8 flex-1 items-center justify-center gap-1.5 rounded-md border border-border text-[12px] text-muted-foreground hover:text-foreground hover:bg-secondary transition-colors"><Icon name="copy" className="h-3.5 w-3.5" />Duplicate</button>
        <button onClick={() => onDelete(node.id)} className="flex h-8 flex-1 items-center justify-center gap-1.5 rounded-md border border-destructive/30 text-[12px] text-destructive hover:bg-destructive/10 transition-colors"><Icon name="trash" className="h-3.5 w-3.5" />Delete</button>
      </div>
    </aside>
  )
}
