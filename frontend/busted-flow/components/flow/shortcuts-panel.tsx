"use client"

export function ShortcutsPanel({ show, onClose }: { show: boolean; onClose: () => void }) {
  if (!show) return null

  const shortcuts = [
    { keys: ["Click + Drag"], desc: "Pan canvas" },
    { keys: ["Mouse Wheel"], desc: "Zoom in/out" },
    { keys: ["Shift", "+", "Drag"], desc: "Snap to grid" },
    { keys: ["Click Handle"], desc: "Start connection" },
    { keys: ["Esc"], desc: "Cancel connection / Deselect" },
    { keys: ["Del"], desc: "Delete selected node/edge" },
    { keys: ["Ctrl", "+", "D"], desc: "Duplicate node" },
    { keys: ["?"], desc: "Show shortcuts" },
  ]

  return (
    <>
      <div className="fixed inset-0 bg-background/80 backdrop-blur-sm z-40 animate-in fade-in duration-200" onClick={onClose} />
      <div className="fixed top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[420px] rounded-lg border border-border bg-card shadow-2xl z-50 animate-in zoom-in-95 slide-in-from-bottom-4 duration-200">
        <div className="flex items-center justify-between border-b border-border px-4 py-3">
          <h3 className="text-sm font-semibold text-foreground">Keyboard Shortcuts</h3>
          <button onClick={onClose} className="rounded-md p-1 hover:bg-secondary transition-colors">
            <svg className="h-4 w-4 text-muted-foreground" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>
        <div className="p-4 space-y-2 max-h-[60vh] overflow-y-auto">
          {shortcuts.map((s, i) => (
            <div key={i} className="flex items-center justify-between py-2 px-2 rounded-md hover:bg-secondary/50 transition-colors">
              <span className="text-[13px] text-muted-foreground">{s.desc}</span>
              <div className="flex items-center gap-1">
                {s.keys.map((k, j) => (
                  <span key={j}>
                    {k === "+" ? <span className="text-[10px] text-muted-foreground/50 mx-0.5">+</span> :
                      <kbd className="rounded border border-border bg-secondary px-2 py-0.5 text-[10px] font-mono text-foreground shadow-sm">{k}</kbd>}
                  </span>
                ))}
              </div>
            </div>
          ))}
        </div>
        <div className="border-t border-border px-4 py-2 text-center">
          <span className="text-[10px] text-muted-foreground">Press <kbd className="rounded border border-border bg-secondary px-1 py-0.5 text-[9px] font-mono">?</kbd> or <kbd className="rounded border border-border bg-secondary px-1 py-0.5 text-[9px] font-mono">Esc</kbd> to close</span>
        </div>
      </div>
    </>
  )
}
