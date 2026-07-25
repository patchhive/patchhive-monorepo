import { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import {
  CommandDialog,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
  CommandSeparator,
} from "@/components/ui/command";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import {
  Activity,
  AlertTriangle,
  Bell,
  BellOff,
  CircleDot,
  Clipboard,
  Command as CmdIcon,
  Cpu,
  Eye,
  EyeOff,
  ExternalLink,
  Gauge,
  GitBranch,
  GitCompare,
  Hexagon,
  KeyRound,
  Keyboard,
  Palette,
  Radio,
  RefreshCw,
  Search,
  Settings,
  Terminal,
  X,
  Zap,
} from "lucide-react";
import { PRODUCTS, type RunEvent, type Status } from "@/lib/hive-data";
import { toast } from "sonner";
import { ShortcutsCheatsheet } from "./shortcuts-cheatsheet";

export type RegistryFilter = "all" | "warn" | "crit";
export type Theme = "amber" | "cyan";

export type AuditKind = "info" | "action" | "destructive" | "ai";
export interface AuditEvent {
  id: string;
  at: number;
  actor: string;
  kind: AuditKind;
  title: string;
  detail?: string;
  diff?: { before: string; after: string };
}

export interface RunbookRun {
  id: string;
  at: number;
  productId: string;
  productName: string;
  dryRun: boolean;
  steps: number;
  actor: string;
}

interface Ctx {
  open: boolean;
  setOpen: (v: boolean | ((p: boolean) => boolean)) => void;
  registryFilter: RegistryFilter;
  setRegistryFilter: (f: RegistryFilter) => void;
  scanlineOn: boolean;
  toggleScanline: () => void;
  pulseProductId: string | null;
  pulseProduct: (id: string) => void;
  repollKey: number;
  repoll: () => void;
  theme: Theme;
  setTheme: (t: Theme) => void;
  cycleTheme: () => void;
  soundOn: boolean;
  toggleSound: () => void;
  cheatsheetOpen: boolean;
  setCheatsheetOpen: (o: boolean) => void;
  replayRun: (run: RunEvent) => void;
  // audit
  auditLog: AuditEvent[];
  logAudit: (e: Omit<AuditEvent, "id" | "at" | "actor"> & { actor?: string }) => void;
  clearAudit: () => void;
  // runbook
  runbookProductId: string | null;
  openRunbook: (id: string) => void;
  closeRunbook: () => void;
  runbookHistory: RunbookRun[];
  recordRunbook: (r: Omit<RunbookRun, "id" | "at" | "actor"> & { actor?: string }) => void;
  // demo/tour/presence
  demoMode: boolean;
  toggleDemo: () => void;
  tourOpen: boolean;
  setTourOpen: (o: boolean) => void;
  presenceOn: boolean;
  togglePresence: () => void;
  // dispatch preview
  dispatchPreviewOpen: boolean;
  setDispatchPreviewOpen: (o: boolean) => void;
  // command palette frecency
  bumpFrecency: (key: string) => void;
  resetFrecency: () => void;
  seedFrecency: () => void;
  frecency: Record<string, { count: number; last: number }>;
}

const HiveCommandContext = createContext<Ctx | null>(null);

export function useHiveCommand() {
  const c = useContext(HiveCommandContext);
  if (!c) throw new Error("useHiveCommand must be used within HiveCommandProvider");
  return c;
}

function playCritSound() {
  try {
    const AC = (window.AudioContext || (window as unknown as { webkitAudioContext: typeof AudioContext }).webkitAudioContext);
    if (!AC) return;
    const ctx = new AC();
    const osc = ctx.createOscillator();
    const gain = ctx.createGain();
    osc.type = "sawtooth";
    osc.frequency.setValueAtTime(660, ctx.currentTime);
    osc.frequency.exponentialRampToValueAtTime(220, ctx.currentTime + 0.5);
    gain.gain.setValueAtTime(0.0001, ctx.currentTime);
    gain.gain.exponentialRampToValueAtTime(0.18, ctx.currentTime + 0.04);
    gain.gain.exponentialRampToValueAtTime(0.0001, ctx.currentTime + 0.55);
    osc.connect(gain).connect(ctx.destination);
    osc.start();
    osc.stop(ctx.currentTime + 0.6);
    osc.onended = () => ctx.close();
  } catch {
    /* no-op */
  }
}

export function HiveCommandProvider({ children }: { children: ReactNode }) {
  const [open, setOpen] = useState(false);
  const [registryFilter, setRegistryFilter] = useState<RegistryFilter>("all");
  const [scanlineOn, setScanlineOn] = useState(true);
  const [pulseProductId, setPulseProductId] = useState<string | null>(null);
  const [repollKey, setRepollKey] = useState(0);
  const [theme, setThemeState] = useState<Theme>("amber");
  const [soundOn, setSoundOn] = useState(false);
  const [cheatsheetOpen, setCheatsheetOpen] = useState(false);
  const [auditLog, setAuditLog] = useState<AuditEvent[]>([]);
  const [runbookProductId, setRunbookProductId] = useState<string | null>(null);
  const [demoMode, setDemoMode] = useState(false);
  const [tourOpen, setTourOpen] = useState(false);
  const [presenceOn, setPresenceOn] = useState(false);

  const [runbookHistory, setRunbookHistory] = useState<RunbookRun[]>([]);
  const [dispatchPreviewOpen, setDispatchPreviewOpen] = useState(false);
  const [frecency, setFrecency] = useState<Record<string, { count: number; last: number }>>({});

  // Load frecency from localStorage
  useEffect(() => {
    try {
      const raw = window.localStorage.getItem("hive.frecency");
      if (raw) setFrecency(JSON.parse(raw));
    } catch { /* ignore */ }
  }, []);

  const bumpFrecency = useCallback((key: string) => {
    setFrecency((prev) => {
      const next = { ...prev, [key]: { count: (prev[key]?.count ?? 0) + 1, last: Date.now() } };
      try { window.localStorage.setItem("hive.frecency", JSON.stringify(next)); } catch { /* ignore */ }
      return next;
    });
  }, []);

  const resetFrecency = useCallback(() => {
    setFrecency({});
    try { window.localStorage.removeItem("hive.frecency"); } catch { /* ignore */ }
  }, []);

  const seedFrecency = useCallback(() => {
    const now = Date.now();
    // A sensible starter set for a first-time operator so the Recent group
    // isn't empty after a reset. Uses the same keys defined by `commands`
    // below (nav.*, filter.*, action.*).
    const seed: Record<string, { count: number; last: number }> = {
      "nav.registry": { count: 6, last: now - 60_000 },
      "nav.runs": { count: 5, last: now - 120_000 },
      "nav.incidents": { count: 4, last: now - 5 * 60_000 },
      "filter.crit": { count: 3, last: now - 10 * 60_000 },
      "action.repoll": { count: 3, last: now - 20 * 60_000 },
    };
    setFrecency(seed);
    try { window.localStorage.setItem("hive.frecency", JSON.stringify(seed)); } catch { /* ignore */ }
  }, []);

  const recordRunbook = useCallback(
    (r: Omit<RunbookRun, "id" | "at" | "actor"> & { actor?: string }) => {
      setRunbookHistory((prev) =>
        [
          { id: Math.random().toString(36).slice(2, 8), at: Date.now(), actor: r.actor ?? "you", ...r },
          ...prev,
        ].slice(0, 30),
      );
    },
    [],
  );

  const logAudit = useCallback((e: Omit<AuditEvent, "id" | "at" | "actor"> & { actor?: string }) => {
    setAuditLog((prev) => [
      { id: Math.random().toString(36).slice(2, 8), at: Date.now(), actor: e.actor ?? "you", ...e },
      ...prev,
    ].slice(0, 50));
  }, []);
  const clearAudit = useCallback(() => setAuditLog([]), []);
  const openRunbook = useCallback((id: string) => setRunbookProductId(id), []);
  const closeRunbook = useCallback(() => setRunbookProductId(null), []);
  const toggleDemo = useCallback(() => setDemoMode((v) => !v), []);
  const togglePresence = useCallback(() => setPresenceOn((v) => !v), []);

  const pulseProduct = useCallback((id: string) => {
    setPulseProductId(id);
    window.setTimeout(() => setPulseProductId((curr) => (curr === id ? null : curr)), 1400);
  }, []);

  const toggleScanline = useCallback(() => setScanlineOn((v) => !v), []);
  const repoll = useCallback(() => setRepollKey((k) => k + 1), []);

  const setTheme = useCallback((t: Theme) => {
    setThemeState(t);
    if (typeof document !== "undefined") {
      document.documentElement.classList.toggle("theme-cyan", t === "cyan");
    }
  }, []);
  const cycleTheme = useCallback(() => {
    setThemeState((curr) => {
      const next: Theme = curr === "amber" ? "cyan" : "amber";
      if (typeof document !== "undefined") {
        document.documentElement.classList.toggle("theme-cyan", next === "cyan");
      }
      return next;
    });
  }, []);
  const toggleSound = useCallback(() => setSoundOn((v) => !v), []);

  // Sync theme to <html> on mount
  useEffect(() => {
    document.documentElement.classList.toggle("theme-cyan", theme === "cyan");
  }, [theme]);

  // Watch crit count transitions; play alarm when count increases.
  const lastCritRef = useRef<number>(PRODUCTS.filter((p) => p.status === "crit").length);
  useEffect(() => {
    if (!soundOn) return;
    const i = window.setInterval(() => {
      const crit = PRODUCTS.filter((p) => p.status === "crit").length;
      if (crit > lastCritRef.current) {
        playCritSound();
        toast.error("CRIT transition detected", { description: `${crit} product${crit === 1 ? "" : "s"} now critical` });
      }
      lastCritRef.current = crit;
    }, 4000);
    return () => window.clearInterval(i);
  }, [soundOn]);

  const replayRun = useCallback((run: RunEvent) => {
    const id = toast.loading(`Replaying ${run.capability}…`, { description: `${run.product} · from ${run.id}` });
    window.setTimeout(() => {
      toast.success("Replay dispatched", {
        id,
        description: `New run queued for ${run.product} · same payload`,
      });
    }, 1100);
  }, []);

  // Global hotkeys
  const gPressedRef = useRef<number>(0);
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const target = e.target as HTMLElement | null;
      const typing =
        target &&
        (target.tagName === "INPUT" || target.tagName === "TEXTAREA" || target.isContentEditable);

      if ((e.key === "k" || e.key === "K") && (e.metaKey || e.ctrlKey)) {
        e.preventDefault();
        setOpen((p) => !p);
        return;
      }
      if (typing) return;

      if (e.key === "/" && !open) {
        e.preventDefault();
        setOpen(true);
      } else if (e.key === "?" || (e.shiftKey && e.key === "/")) {
        e.preventDefault();
        setCheatsheetOpen(true);
      } else if (e.key === "t") {
        cycleTheme();
      } else if (e.key === "s") {
        toggleScanline();
      } else if (e.key === "m") {
        setSoundOn((v) => !v);
      } else if (e.key === "g") {
        gPressedRef.current = Date.now();
      } else if (Date.now() - gPressedRef.current < 800) {
        // sequence "g <letter>"
        const map: Record<string, string> = {
          d: "deck",
          r: "registry",
          u: "runs",
          c: "capabilities",
          t: "tokens",
          i: "incidents",
          s: "slo",
          x: "drift",
          p: "products",
          q: "cap-search",
          y: "dependencies",
        };
        const target = map[e.key.toLowerCase()];
        if (target) {
          const el = document.getElementById(target);
          el?.scrollIntoView({ behavior: "smooth", block: "start" });
        }
        gPressedRef.current = 0;
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open, cycleTheme, toggleScanline]);

  const value = useMemo<Ctx>(
    () => ({
      open,
      setOpen,
      registryFilter,
      setRegistryFilter,
      scanlineOn,
      toggleScanline,
      pulseProductId,
      pulseProduct,
      repollKey,
      repoll,
      theme,
      setTheme,
      cycleTheme,
      soundOn,
      toggleSound,
      cheatsheetOpen,
      setCheatsheetOpen,
      replayRun,
      auditLog,
      logAudit,
      clearAudit,
      runbookProductId,
      openRunbook,
      closeRunbook,
      demoMode,
      toggleDemo,
      tourOpen,
      setTourOpen,
      presenceOn,
      togglePresence,
      runbookHistory,
      recordRunbook,
      dispatchPreviewOpen,
      setDispatchPreviewOpen,
      bumpFrecency,
      resetFrecency,
      seedFrecency,
      frecency,
    }),
    [open, registryFilter, scanlineOn, pulseProductId, pulseProduct, toggleScanline, repollKey, repoll, theme, setTheme, cycleTheme, soundOn, toggleSound, cheatsheetOpen, replayRun, auditLog, logAudit, clearAudit, runbookProductId, openRunbook, closeRunbook, demoMode, toggleDemo, tourOpen, presenceOn, togglePresence, runbookHistory, recordRunbook, dispatchPreviewOpen, bumpFrecency, resetFrecency, seedFrecency, frecency],
  );

  return (
    <HiveCommandContext.Provider value={value}>
      {children}
      <ShortcutsCheatsheet open={cheatsheetOpen} onOpenChange={setCheatsheetOpen} />
    </HiveCommandContext.Provider>
  );
}

function scrollToId(id: string) {
  const el = document.getElementById(id);
  if (!el) return;
  el.scrollIntoView({ behavior: "smooth", block: "start" });
  el.classList.add("ring-2", "ring-[var(--honey)]");
  window.setTimeout(() => el.classList.remove("ring-2", "ring-[var(--honey)]"), 1200);
}

const statusDotCls: Record<Status, string> = {
  ok: "bg-[var(--ok)]",
  warn: "bg-[var(--warn)]",
  crit: "bg-[var(--crit)]",
  offline: "bg-muted-foreground",
};

export function HiveCommand() {
  const {
    open,
    setOpen,
    registryFilter,
    setRegistryFilter,
    toggleScanline,
    scanlineOn,
    pulseProduct,
    repoll,
    theme,
    cycleTheme,
    soundOn,
    toggleSound,
    setCheatsheetOpen,
    bumpFrecency,
    resetFrecency,
    seedFrecency,
    frecency,
  } = useHiveCommand();

  const filterLabel: Record<RegistryFilter, string> = {
    all: "All products",
    warn: "Degraded only",
    crit: "Down only",
  };

  const applyFilter = useCallback(
    (next: RegistryFilter) => {
      const prev = registryFilter;
      setRegistryFilter(next);
      scrollToId("registry");
      if (prev === next) return;
      toast(`Filter: ${filterLabel[next]}`, {
        description: `Was: ${filterLabel[prev]}`,
        action: { label: "Undo", onClick: () => setRegistryFilter(prev) },
      });
    },
    [registryFilter, setRegistryFilter],
  );

  const dispatchBootstrap = useCallback(() => {
    let cancelled = false;
    const t = window.setTimeout(() => {
      if (cancelled) return;
      toast.success("Bootstrap complete", { description: "All nodes warm" });
    }, 2200);
    toast.warning("Bootstrap dispatched", {
      description: "Suite warm-up in progress · 2s window to abort",
      action: {
        label: "Abort",
        onClick: () => {
          cancelled = true;
          window.clearTimeout(t);
          toast("Bootstrap aborted");
        },
      },
    });
  }, []);

  const flipScanline = useCallback(() => {
    toggleScanline();
    const nowOn = !scanlineOn;
    toast(nowOn ? "Scanline on" : "Scanline off", {
      action: { label: "Undo", onClick: () => toggleScanline() },
    });
  }, [toggleScanline, scanlineOn]);

  const flipTheme = useCallback(() => {
    cycleTheme();
    const next: Theme = theme === "amber" ? "cyan" : "amber";
    toast(`Theme: ${next}`, { description: next === "cyan" ? "Cold operator palette" : "Honey amber palette" });
  }, [cycleTheme, theme]);

  const flipSound = useCallback(() => {
    toggleSound();
    toast(soundOn ? "Crit alarm muted" : "Crit alarm armed", {
      description: soundOn ? "No sound on ok→crit transitions" : "Plays a tone when a product flips to crit",
    });
  }, [toggleSound, soundOn]);

  interface Confirm {
    title: string;
    description: string;
    confirmLabel: string;
    tone: "destructive" | "default";
    onConfirm: () => void;
  }
  const [pending, setPending] = useState<Confirm | null>(null);

  const requestConfirm = useCallback(
    (c: Confirm) => {
      setOpen(false);
      setPending(c);
    },
    [setOpen],
  );

  type Group = "Navigate" | "Products" | "Actions" | "Filters";
  interface Cmd {
    key: string;
    group: Group;
    label: string;
    value: string;
    icon: typeof CmdIcon;
    hint?: React.ReactNode;
    tail?: React.ReactNode;
    run: () => void;
  }

  const navItems: { id: string; label: string; icon: typeof CmdIcon }[] = [
    { id: "deck", label: "Deck", icon: CmdIcon },
    { id: "registry", label: "Registry", icon: Radio },
    { id: "runs", label: "Runs", icon: Terminal },
    { id: "incidents", label: "Incidents", icon: AlertTriangle },
    { id: "dependencies", label: "Dependencies", icon: GitBranch },
    { id: "slo", label: "SLOs", icon: Gauge },
    { id: "drift", label: "Contract drift", icon: GitCompare },
    { id: "cap-search", label: "Capability search", icon: Search },
    { id: "tokens", label: "Token vault", icon: KeyRound },
    { id: "capabilities", label: "Capabilities", icon: Cpu },
    { id: "products", label: "Products vs Mesh", icon: Hexagon },
    { id: "setup", label: "Setup", icon: Settings },
  ];

  const commands: Cmd[] = useMemo(() => {
    const out: Cmd[] = [];
    for (const n of navItems) {
      out.push({
        key: `nav:${n.id}`,
        group: "Navigate",
        label: n.label,
        value: `nav ${n.label}`,
        icon: n.icon,
        tail: <span className="font-display text-[10px] uppercase tracking-wider text-muted-foreground">#{n.id}</span>,
        run: () => scrollToId(n.id),
      });
    }
    for (const p of PRODUCTS) {
      out.push({
        key: `product:${p.id}`,
        group: "Products",
        label: p.name,
        value: `product ${p.name} ${p.tagline}`,
        icon: Hexagon,
        hint: <span className="ml-2 truncate text-xs text-muted-foreground">{p.tagline}</span>,
        tail: (
          <span className="flex items-center gap-2">
            <span className="font-display text-[10px] text-muted-foreground">
              {p.status === "crit" ? "—" : `${p.latencyMs}ms`}
            </span>
            <span className={`h-1.5 w-1.5 rounded-full ${statusDotCls[p.status]}`} />
          </span>
        ),
        run: () => {
          scrollToId(`product-${p.id}`);
          pulseProduct(p.id);
        },
      });
    }
    out.push(
      {
        key: "action:bootstrap",
        group: "Actions",
        label: "Dispatch suite bootstrap",
        value: "action bootstrap suite dispatch",
        icon: Zap,
        tail: <span className="font-display text-[9px] uppercase tracking-wider text-[var(--crit)]">confirm</span>,
        run: () =>
          requestConfirm({
            title: "Dispatch suite bootstrap?",
            description: "Warms every node in the mesh. Active sessions may see brief latency spikes while caches rebuild.",
            confirmLabel: "Dispatch bootstrap",
            tone: "destructive",
            onConfirm: dispatchBootstrap,
          }),
      },
      {
        key: "action:repoll",
        group: "Actions",
        label: "Re-poll /health",
        value: "action repoll health refresh",
        icon: RefreshCw,
        tail: <span className="font-display text-[9px] uppercase tracking-wider text-[var(--crit)]">confirm</span>,
        run: () =>
          requestConfirm({
            title: "Re-poll /health across the mesh?",
            description: "Forces every product to respond to a fresh health probe. Generates traffic on all services.",
            confirmLabel: "Re-poll now",
            tone: "destructive",
            onConfirm: () => {
              repoll();
              toast("Re-polling /health across the mesh");
            },
          }),
      },
      {
        key: "action:copy-registry",
        group: "Actions",
        label: "Copy registry as JSON",
        value: "action copy registry json clipboard",
        icon: Clipboard,
        run: () => {
          const payload = JSON.stringify(
            PRODUCTS.map((p) => ({ id: p.id, status: p.status, latencyMs: p.latencyMs, uptime: p.uptime })),
            null,
            2,
          );
          navigator.clipboard?.writeText(payload);
          toast.success("Registry snapshot copied", { description: `${PRODUCTS.length} products` });
        },
      },
      {
        key: "action:scanline",
        group: "Actions",
        label: "Toggle scanline overlay",
        value: "action toggle scanline overlay",
        icon: scanlineOn ? EyeOff : Eye,
        tail: <span className="font-display text-[9px] uppercase tracking-wider text-muted-foreground">s</span>,
        run: flipScanline,
      },
      {
        key: "action:theme",
        group: "Actions",
        label: `Cycle theme · currently ${theme}`,
        value: "action cycle theme amber cyan",
        icon: Palette,
        tail: <span className="font-display text-[9px] uppercase tracking-wider text-muted-foreground">t</span>,
        run: flipTheme,
      },
      {
        key: "action:sound",
        group: "Actions",
        label: soundOn ? "Mute crit alarm" : "Arm crit alarm",
        value: "action toggle crit sound alarm",
        icon: soundOn ? BellOff : Bell,
        tail: <span className="font-display text-[9px] uppercase tracking-wider text-muted-foreground">m</span>,
        run: flipSound,
      },
      {
        key: "action:cheatsheet",
        group: "Actions",
        label: "Show keyboard shortcuts",
        value: "action show keyboard shortcuts cheatsheet help",
        icon: Keyboard,
        tail: <span className="font-display text-[9px] uppercase tracking-wider text-muted-foreground">?</span>,
        run: () => setCheatsheetOpen(true),
      },
      {
        key: "action:open-frontend",
        group: "Actions",
        label: `Open ${PRODUCTS[0].name} frontend`,
        value: "action open frontend top product",
        icon: ExternalLink,
        run: () => window.open(PRODUCTS[0].frontend, "_blank", "noopener"),
      },
      {
        key: "filter:warn",
        group: "Filters",
        label: "Show only degraded",
        value: "filter degraded warn",
        icon: AlertTriangle,
        run: () => applyFilter("warn"),
      },
      {
        key: "filter:crit",
        group: "Filters",
        label: "Show only down",
        value: "filter down crit critical",
        icon: CircleDot,
        run: () => applyFilter("crit"),
      },
      {
        key: "filter:all",
        group: "Filters",
        label: "Clear filters",
        value: "filter clear all",
        icon: X,
        run: () => applyFilter("all"),
      },
      {
        key: "nav:overview",
        group: "Filters",
        label: "Suite overview",
        value: "status overview",
        icon: Activity,
        run: () => scrollToId("deck"),
      },
    );
    return out;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [scanlineOn, soundOn, theme, applyFilter, dispatchBootstrap, flipScanline, flipSound, flipTheme, pulseProduct, repoll, requestConfirm, setCheatsheetOpen]);

  const cmdByKey = useMemo(() => {
    const m = new Map<string, Cmd>();
    commands.forEach((c) => m.set(c.key, c));
    return m;
  }, [commands]);

  const invoke = useCallback(
    (c: Cmd) => {
      bumpFrecency(c.key);
      c.run();
      setOpen(false);
    },
    [bumpFrecency, setOpen],
  );

  const recent: Cmd[] = useMemo(() => {
    const now = Date.now();
    return Object.entries(frecency)
      .map(([k, v]) => {
        const hours = Math.max(0, (now - v.last) / 3_600_000);
        return { k, score: v.count / (1 + hours * 0.5) };
      })
      .sort((a, b) => b.score - a.score)
      .slice(0, 5)
      .map((e) => cmdByKey.get(e.k))
      .filter((c): c is Cmd => !!c);
  }, [frecency, cmdByKey]);

  const groupOf = (g: Group) => commands.filter((c) => c.group === g);

  const renderItem = (c: Cmd, keyPrefix = "") => (
    <CommandItem key={`${keyPrefix}${c.key}`} value={c.value} onSelect={() => invoke(c)}>
      <c.icon className="mr-2 h-4 w-4 text-[var(--honey)]" />
      <span>{c.label}</span>
      {c.hint}
      {c.tail && <span className="ml-auto">{c.tail}</span>}
    </CommandItem>
  );

  return (
    <>
      <CommandDialog open={open} onOpenChange={setOpen}>
        <CommandInput placeholder="Search commands, products, actions…" />
        <CommandList className="max-h-[60vh]">
          <CommandEmpty>No matches in the hive.</CommandEmpty>

          {recent.length > 0 && (
            <>
              <CommandGroup heading="Recent">
                {recent.map((c) => renderItem(c, "recent-"))}
              </CommandGroup>
              <CommandSeparator />
            </>
          )}

          <CommandGroup heading="Navigate">{groupOf("Navigate").map((c) => renderItem(c))}</CommandGroup>
          <CommandSeparator />
          <CommandGroup heading="Products">{groupOf("Products").map((c) => renderItem(c))}</CommandGroup>
          <CommandSeparator />
          <CommandGroup heading="Actions">{groupOf("Actions").map((c) => renderItem(c))}</CommandGroup>
          <CommandSeparator />
          <CommandGroup heading="Filters">{groupOf("Filters").map((c) => renderItem(c))}</CommandGroup>
        </CommandList>
        <div className="flex items-center justify-between gap-3 border-t border-border/60 px-3 py-2 font-display text-[10px] uppercase tracking-wider text-muted-foreground">
          <span className="truncate">↑↓ navigate · ↵ run · ? cheatsheet · esc close</span>
          <div className="flex items-center gap-2 whitespace-nowrap">
            <span className="hidden text-muted-foreground/70 sm:inline">
              recent: {Object.keys(frecency).length}
            </span>
            <button
              type="button"
              onClick={() => {
                seedFrecency();
                toast("Recent seeded", { description: "Frecency reset to starter defaults" });
              }}
              className="rounded border border-border bg-card/60 px-1.5 py-0.5 transition hover:border-[var(--honey)]/50 hover:text-[var(--honey)]"
              title="Reset frecency to a curated starter set"
            >
              Seed
            </button>
            <button
              type="button"
              onClick={() => {
                resetFrecency();
                toast("Recent cleared", { description: "Command frecency wiped" });
              }}
              disabled={Object.keys(frecency).length === 0}
              className="rounded border border-border bg-card/60 px-1.5 py-0.5 transition hover:border-[var(--crit)]/50 hover:text-[var(--crit)] disabled:opacity-40"
              title="Clear frecency scores from localStorage"
            >
              Reset
            </button>
            <span className="text-[var(--honey)]">hivecore</span>
          </div>
        </div>
      </CommandDialog>
      <AlertDialog open={pending !== null} onOpenChange={(o) => !o && setPending(null)}>
        <AlertDialogContent className="border-[var(--honey)]/40 bg-background/95">
          <AlertDialogHeader>
            <AlertDialogTitle className="font-display uppercase tracking-wider text-[var(--honey)]">
              {pending?.title}
            </AlertDialogTitle>
            <AlertDialogDescription>{pending?.description}</AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction
              className={
                pending?.tone === "destructive"
                  ? "bg-[var(--crit)] text-background hover:bg-[var(--crit)]/90"
                  : undefined
              }
              onClick={() => {
                const p = pending;
                setPending(null);
                p?.onConfirm();
              }}
            >
              {pending?.confirmLabel ?? "Confirm"}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </>
  );
}
