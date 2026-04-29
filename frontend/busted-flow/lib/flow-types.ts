export type NodeCategory = "entry" | "durable" | "state" | "flow" | "timing" | "signal"
export type ServiceType = "service" | "virtual-object" | "workflow"
export type InvocationStatus = "pending" | "running" | "suspended" | "completed" | "failed" | "retrying"

export interface Position { x: number; y: number }

export interface NodeData {
  label: string
  description: string
  icon: string
  category: NodeCategory
  configured: boolean
  status?: InvocationStatus
  durableStepName?: string
  retryCount?: number
  journalIndex?: number
  stateKey?: string
  targetService?: string
  targetHandler?: string
  targetMethod?: string
  promiseName?: string
  awakeableId?: string
  sleepDuration?: string
  idempotencyKey?: string
  conditionExpression?: string
  loopIterator?: string
  errorHandler?: string
  compensationHandler?: string
  timeoutMs?: number
  topic?: string
  cronExpression?: string
  workflowKey?: string
  payload?: string
}

export interface FlowNode { id: string; position: Position; data: NodeData }
export interface FlowEdge { id: string; source: string; target: string; label?: string; animated?: boolean }
export interface NodeTemplate { type: string; label: string; description: string; icon: string; category: NodeCategory }

export const NODE_WIDTH = 240
export const NODE_HEIGHT = 72

export const nodeTemplates: NodeTemplate[] = [
  { type: "http-handler",     label: "HTTP Handler",     description: "Handle HTTP/gRPC invocation",       icon: "globe",        category: "entry" },
  { type: "kafka-handler",    label: "Kafka Consumer",   description: "Consume events from Kafka topic",   icon: "kafka",        category: "entry" },
  { type: "cron-trigger",     label: "Cron Trigger",     description: "Schedule periodic execution",       icon: "clock",        category: "entry" },
  { type: "workflow-submit",  label: "Workflow Submit",   description: "Submit workflow with key",          icon: "play-circle",  category: "entry" },
  { type: "run",              label: "Durable Step",     description: "ctx.run() - persisted side effect",  icon: "shield",       category: "durable" },
  { type: "service-call",     label: "Service Call",     description: "Request-response to service",        icon: "arrow-right",  category: "durable" },
  { type: "object-call",      label: "Object Call",      description: "Call virtual object handler",        icon: "box",          category: "durable" },
  { type: "workflow-call",    label: "Workflow Call",     description: "Submit or attach to workflow",       icon: "workflow",     category: "durable" },
  { type: "send-message",     label: "Send Message",     description: "Fire-and-forget one-way call",       icon: "send",         category: "durable" },
  { type: "delayed-send",     label: "Delayed Message",  description: "Schedule future handler call",       icon: "clock-send",   category: "durable" },
  { type: "get-state",        label: "Get State",        description: "ctx.get() - read persisted state",   icon: "download",     category: "state" },
  { type: "set-state",        label: "Set State",        description: "ctx.set() - write persisted state",  icon: "upload",       category: "state" },
  { type: "clear-state",      label: "Clear State",      description: "ctx.clear() / clearAll()",           icon: "eraser",       category: "state" },
  { type: "condition",        label: "If / Else",        description: "Conditional branching",              icon: "git-branch",   category: "flow" },
  { type: "switch",           label: "Switch",           description: "Multi-path routing",                 icon: "git-fork",     category: "flow" },
  { type: "loop",             label: "Loop / Iterate",   description: "Iterate over collection",            icon: "repeat",       category: "flow" },
  { type: "parallel",         label: "Parallel",         description: "Promise.all() concurrent steps",     icon: "layers",       category: "flow" },
  { type: "compensate",       label: "Compensate",       description: "Saga compensation / rollback",       icon: "undo",         category: "flow" },
  { type: "sleep",            label: "Sleep / Timer",    description: "ctx.sleep() - durable pause",        icon: "timer",        category: "timing" },
  { type: "timeout",          label: "Timeout",          description: "orTimeout() - deadline guard",       icon: "alarm",        category: "timing" },
  { type: "durable-promise",  label: "Durable Promise",  description: "ctx.promise() - await external",    icon: "sparkles",     category: "signal" },
  { type: "awakeable",        label: "Awakeable",        description: "Pause for external completion",      icon: "bell",         category: "signal" },
  { type: "resolve-promise",  label: "Resolve Promise",  description: "Resolve a durable promise",          icon: "check-circle", category: "signal" },
  { type: "signal-handler",   label: "Signal Handler",   description: "Shared handler for signals",         icon: "radio",        category: "signal" },
]

export const categoryLabels: Record<NodeCategory, string> = {
  entry: "Entry Points", durable: "Durable Steps", state: "State",
  flow: "Control Flow", timing: "Timing & Events", signal: "Signals & Promises",
}

export const categoryOrder: NodeCategory[] = ["entry", "durable", "state", "flow", "timing", "signal"]
