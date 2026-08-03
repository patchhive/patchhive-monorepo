import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from "@/components/ui/dialog";

const groups: { heading: string; rows: { keys: string[]; label: string }[] }[] = [
  {
    heading: "Palette",
    rows: [
      { keys: ["⌘", "K"], label: "Open command palette" },
      { keys: ["Ctrl", "K"], label: "Open command palette (PC)" },
      { keys: ["/"], label: "Open command palette" },
      { keys: ["esc"], label: "Close palette / drawer" },
    ],
  },
  {
    heading: "Navigation",
    rows: [
      { keys: ["g", "d"], label: "Jump to Deck" },
      { keys: ["g", "r"], label: "Jump to Registry" },
      { keys: ["g", "u"], label: "Jump to Runs" },
      { keys: ["g", "t"], label: "Jump to Tokens" },
      { keys: ["g", "i"], label: "Jump to Incidents" },
      { keys: ["g", "s"], label: "Jump to Suite runs" },
      { keys: ["g", "c"], label: "Jump to Controls" },
      { keys: ["g", "p"], label: "Jump to Approvals" },
      { keys: ["g", "b"], label: "Jump to Bootstrap" },
    ],
  },
  {
    heading: "Display",
    rows: [
      { keys: ["t"], label: "Cycle theme (amber ↔ cyan)" },
      { keys: ["s"], label: "Toggle scanline overlay" },
      { keys: ["m"], label: "Mute / unmute crit alarm" },
      { keys: ["?"], label: "Show this cheatsheet" },
    ],
  },
];

interface Props {
  open: boolean;
  onOpenChange: (o: boolean) => void;
}

export function ShortcutsCheatsheet({ open, onOpenChange }: Props) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-2xl border-[var(--honey)]/40 bg-background/95">
        <DialogHeader>
          <DialogTitle className="font-display uppercase tracking-wider text-[var(--honey)]">
            Keyboard shortcuts
          </DialogTitle>
          <DialogDescription>
            Operator-deck bindings. Press <Kbd>?</Kbd> any time to reopen this list.
          </DialogDescription>
        </DialogHeader>
        <div className="grid gap-5 sm:grid-cols-3">
          {groups.map((g) => (
            <div key={g.heading}>
              <div className="mb-2 font-display text-[10px] uppercase tracking-[0.2em] text-[var(--honey)]/80">
                {g.heading}
              </div>
              <ul className="space-y-1.5">
                {g.rows.map((r) => (
                  <li key={r.label} className="flex items-center justify-between gap-3 text-xs">
                    <span className="text-foreground/90">{r.label}</span>
                    <span className="flex gap-1">
                      {r.keys.map((k, i) => (
                        <Kbd key={i}>{k}</Kbd>
                      ))}
                    </span>
                  </li>
                ))}
              </ul>
            </div>
          ))}
        </div>
      </DialogContent>
    </Dialog>
  );
}

function Kbd({ children }: { children: React.ReactNode }) {
  return (
    <kbd className="rounded border border-border bg-card/80 px-1.5 py-0.5 font-display text-[10px] text-foreground shadow-sm">
      {children}
    </kbd>
  );
}
