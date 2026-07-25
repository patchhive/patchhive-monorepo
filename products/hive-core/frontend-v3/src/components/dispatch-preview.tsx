import { useMemo } from "react";
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
import { Rocket } from "lucide-react";
import { PRODUCTS } from "@/lib/hive-data";
import { DEPENDENCIES } from "@/lib/hive-extra";
import { toast } from "sonner";

function downstreams(id: string): string[] {
  const seen = new Set<string>();
  const q = [id];
  while (q.length) {
    const cur = q.shift()!;
    for (const e of DEPENDENCIES) {
      if (e.from === cur && !seen.has(e.to) && e.to !== id) {
        seen.add(e.to);
        q.push(e.to);
      }
    }
  }
  return [...seen];
}

interface Props {
  open: boolean;
  onOpenChange: (o: boolean) => void;
}

export function DispatchPreview({ open, onOpenChange }: Props) {
  // Default preview: bootstrap = touches all products from hivecore
  const targetId = "hivecore";
  const affected = useMemo(() => downstreams(targetId), []);
  const byId = Object.fromEntries(PRODUCTS.map((p) => [p.id, p]));
  const critAffected = affected.filter((id) => byId[id]?.status === "crit").length;

  return (
    <AlertDialog open={open} onOpenChange={onOpenChange}>
      <AlertDialogContent className="border-[var(--honey)]/40 bg-background/95 sm:max-w-lg">
        <AlertDialogHeader>
          <AlertDialogTitle className="flex items-center gap-2 font-display uppercase tracking-wider text-[var(--honey)]">
            <Rocket className="h-4 w-4" /> Dispatch preview · Blast radius
          </AlertDialogTitle>
          <AlertDialogDescription>
            HiveCore will fan out to <span className="text-[var(--honey)] font-semibold">{affected.length}</span> downstream
            product{affected.length === 1 ? "" : "s"} across the mesh.
            {critAffected > 0 && (
              <>
                {" "}
                <span className="text-[var(--crit)] font-semibold">{critAffected}</span> currently critical — dispatch may amplify pressure.
              </>
            )}
          </AlertDialogDescription>
        </AlertDialogHeader>

        <div className="my-2 flex flex-wrap gap-1.5">
          {affected.map((id) => {
            const p = byId[id];
            if (!p) return null;
            const tone =
              p.status === "crit"
                ? "var(--crit)"
                : p.status === "warn"
                  ? "var(--warn)"
                  : "var(--ok)";
            return (
              <span
                key={id}
                className="inline-flex items-center gap-1.5 rounded border border-border bg-background/60 px-2 py-0.5 font-display text-[10px]"
              >
                <span
                  className="h-1.5 w-1.5 rounded-full"
                  style={{ background: tone, boxShadow: `0 0 6px ${tone}` }}
                />
                {p.name}
              </span>
            );
          })}
        </div>

        <AlertDialogFooter>
          <AlertDialogCancel>Cancel</AlertDialogCancel>
          <AlertDialogAction
            className="bg-[var(--honey)] text-background hover:bg-[var(--honey)]/90"
            onClick={() => toast.success("Dispatch queued", { description: `${affected.length} downstreams notified` })}
          >
            Confirm dispatch
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}
