import { Outlet, createRootRoute } from "@tanstack/react-router";

function NotFound() {
  return (
    <div className="flex min-h-screen items-center justify-center px-4">
      <div className="max-w-md text-center">
        <h1 className="font-display text-6xl font-bold text-[var(--honey)]">404</h1>
        <p className="mt-3 text-sm text-muted-foreground">
          No such deck surface. The control plane runs on a single route.
        </p>
      </div>
    </div>
  );
}

function ErrorPanel({ error }: { error: Error }) {
  return (
    <div className="flex min-h-screen items-center justify-center px-4">
      <div className="max-w-lg rounded-xl border border-[var(--crit)]/40 bg-card/60 p-6">
        <h1 className="font-display text-sm font-bold uppercase tracking-wider text-[var(--crit)]">
          Deck failed to render
        </h1>
        <p className="mt-2 text-sm text-muted-foreground">{error.message}</p>
      </div>
    </div>
  );
}

export const Route = createRootRoute({
  component: () => <Outlet />,
  notFoundComponent: NotFound,
  errorComponent: ErrorPanel,
});
