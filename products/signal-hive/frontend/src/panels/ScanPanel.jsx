import { useMemo, useState } from "react";
import { Btn, EmptyState, Input, S, Sel } from "@patchhivehq/ui";
import SignalCard from "../components/SignalCard.jsx";
import { SORT_OPTIONS, sortRepos } from "../sort.js";

export default function ScanPanel({ params, setParams, running, onRun, scan }) {
  const [sortBy, setSortBy] = useState("priority");
  const set = (key, value) => setParams((prev) => ({ ...prev, [key]: value }));
  const sortedRepos = useMemo(() => sortRepos(scan?.repos || [], sortBy), [scan, sortBy]);

  return (
    <div style={{ display: "grid", gap: 18 }}>
      <div style={{ ...S.panel, display: "grid", gap: 16 }}>
        <div style={{ display: "grid", gap: 6 }}>
          <div style={{ fontSize: 24, fontWeight: 700, color: "var(--accent)" }}>
            See the maintenance work your team is missing.
          </div>
          <div style={{ color: "var(--text-dim)", fontSize: 13, lineHeight: 1.6, maxWidth: 900 }}>
            SignalHive analyzes repository and issue history to surface stale risks, duplicate problems,
            TODO/FIXME hotspots, and backlog drag before they slow delivery. This first slice is read-only
            and intentionally focused on visibility first.
          </div>
        </div>

        <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(220px, 1fr))", gap: 12 }}>
          <div style={S.field}>
            <div style={S.label}>Search Query</div>
            <Input value={params.search_query} onChange={(value) => set("search_query", value)} placeholder="bug triage, maintenance, backlog" />
          </div>
          <div style={S.field}>
            <div style={S.label}>Topics</div>
            <Input value={params.topics} onChange={(value) => set("topics", value)} placeholder="payments, api, maintenance" />
          </div>
          <div style={S.field}>
            <div style={S.label}>Languages</div>
            <Input value={params.languages} onChange={(value) => set("languages", value)} placeholder="rust,typescript,python" />
          </div>
          <div style={S.field}>
            <div style={S.label}>Min Stars</div>
            <Input value={params.min_stars} onChange={(value) => set("min_stars", value)} placeholder="25" type="number" />
          </div>
          <div style={S.field}>
            <div style={S.label}>Max Repos</div>
            <Input value={params.max_repos} onChange={(value) => set("max_repos", value)} placeholder="8" type="number" />
          </div>
          <div style={S.field}>
            <div style={S.label}>Issues / Repo</div>
            <Input value={params.issues_per_repo} onChange={(value) => set("issues_per_repo", value)} placeholder="30" type="number" />
          </div>
          <div style={S.field}>
            <div style={S.label}>Stale Threshold (days)</div>
            <Input value={params.stale_days} onChange={(value) => set("stale_days", value)} placeholder="45" type="number" />
          </div>
        </div>

        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", gap: 12, flexWrap: "wrap" }}>
          <div style={{ color: "var(--text-muted)", fontSize: 11, lineHeight: 1.5 }}>
            SignalHive only reads GitHub metadata and code search results. It does not clone repos or
            write anything. If you set an allowlist in Controls, that list can drive scans even when
            the search fields are blank.
          </div>
          <Btn onClick={onRun} disabled={running}>
            {running ? "Scanning…" : "Run Signal Scan"}
          </Btn>
        </div>
      </div>

      {!scan && <EmptyState icon="📡" text="Run your first SignalHive scan to generate a ranked maintenance queue." />}

      {scan && (
        <div style={{ display: "grid", gap: 16 }}>
          <div style={{ ...S.panel, display: "flex", justifyContent: "space-between", gap: 12, flexWrap: "wrap" }}>
            <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
              <div style={{ fontSize: 18, fontWeight: 700 }}>Latest Ranked Queue</div>
              <div style={{ color: "var(--text-dim)", fontSize: 12 }}>
                {scan.summary.total_repos} repos scanned • {scan.summary.total_signals} signals found • top repo {scan.summary.top_repo}
              </div>
            </div>
            <div style={{ display: "flex", gap: 12, alignItems: "flex-end", flexWrap: "wrap" }}>
              <div style={{ minWidth: 180, ...S.field }}>
                <div style={S.label}>Sort Queue</div>
                <Sel value={sortBy} onChange={setSortBy} opts={SORT_OPTIONS} />
              </div>
              <div style={{ color: "var(--text-muted)", fontSize: 11 }}>
                Scan ID {scan.id}
              </div>
            </div>
          </div>

          {sortedRepos.length === 0 ? (
            <EmptyState icon="◌" text="No repositories matched strongly enough to rank in this scan." />
          ) : (
            sortedRepos.map((repo) => <SignalCard key={repo.full_name} repo={repo} />)
          )}
        </div>
      )}
    </div>
  );
}
