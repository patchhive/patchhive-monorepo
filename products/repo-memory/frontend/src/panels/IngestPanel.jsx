import { Btn, EmptyState, Input, S, Tag } from "@patchhivehq/ui";

async function copyText(text) {
  if (!navigator?.clipboard?.writeText) return false;
  await navigator.clipboard.writeText(text);
  return true;
}

export default function IngestPanel({ form, setForm, running, onRun, run, onShowMemory }) {
  const update = (key, value) => setForm((prev) => ({ ...prev, [key]: value }));

  return (
    <div style={{ display: "grid", gap: 18 }}>
      <div style={{ ...S.panel, display: "grid", gap: 14 }}>
        <div style={{ fontSize: 18, fontWeight: 700 }}>Ingest merged history</div>
        <div style={{ color: "var(--text-dim)", lineHeight: 1.6 }}>
          RepoMemory pulls recent merged PRs, review feedback, and closed issues, then turns them into reusable repo-specific memory entries.
        </div>
        <div style={{ display: "grid", gridTemplateColumns: "2fr repeat(3, 1fr)", gap: 12 }}>
          <div style={S.field}>
            <div style={S.label}>Repo</div>
            <Input value={form.repo} onChange={(value) => update("repo", value)} placeholder="owner/repo" />
          </div>
          <div style={S.field}>
            <div style={S.label}>Merged PRs</div>
            <Input value={form.merged_pr_limit} onChange={(value) => update("merged_pr_limit", value)} type="number" />
          </div>
          <div style={S.field}>
            <div style={S.label}>Closed issues</div>
            <Input value={form.issue_limit} onChange={(value) => update("issue_limit", value)} type="number" />
          </div>
          <div style={S.field}>
            <div style={S.label}>Lookback days</div>
            <Input value={form.since_days} onChange={(value) => update("since_days", value)} type="number" />
          </div>
        </div>
        <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
          <Btn onClick={onRun} disabled={running || !form.repo.trim()}>
            {running ? "Ingesting..." : "Build memory"}
          </Btn>
          <Tag color="var(--green)">merged PRs</Tag>
          <Tag color="var(--gold)">review feedback</Tag>
          <Tag color="var(--blue)">closed bugs</Tag>
        </div>
      </div>

      {run ? (
        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 16 }}>
          <div style={{ ...S.panel, display: "grid", gap: 10 }}>
            <div style={{ display: "flex", justifyContent: "space-between", gap: 8, alignItems: "center", flexWrap: "wrap" }}>
              <div>
                <div style={{ fontSize: 16, fontWeight: 700 }}>{run.repo}</div>
                <div style={{ color: "var(--text-dim)", fontSize: 11 }}>{run.summary.top_memory}</div>
              </div>
              <Btn onClick={onShowMemory} style={{ padding: "4px 10px" }}>
                Open memory
              </Btn>
            </div>
            <div style={{ display: "grid", gridTemplateColumns: "repeat(2, minmax(0, 1fr))", gap: 10 }}>
              <div style={{ border: "1px solid var(--border)", borderRadius: 8, padding: 10 }}>
                <div style={S.label}>Merged PRs</div>
                <div style={{ fontSize: 18, fontWeight: 700 }}>{run.summary.merged_prs_analyzed}</div>
              </div>
              <div style={{ border: "1px solid var(--border)", borderRadius: 8, padding: 10 }}>
                <div style={S.label}>Review feedback</div>
                <div style={{ fontSize: 18, fontWeight: 700 }}>{run.summary.review_feedback_items}</div>
              </div>
              <div style={{ border: "1px solid var(--border)", borderRadius: 8, padding: 10 }}>
                <div style={S.label}>Closed issues</div>
                <div style={{ fontSize: 18, fontWeight: 700 }}>{run.summary.closed_issues_analyzed}</div>
              </div>
              <div style={{ border: "1px solid var(--border)", borderRadius: 8, padding: 10 }}>
                <div style={S.label}>Memories created</div>
                <div style={{ fontSize: 18, fontWeight: 700 }}>{run.summary.memories_created}</div>
              </div>
            </div>
            <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
              <Tag color="var(--green)">{run.summary.conventions} conventions</Tag>
              <Tag color="var(--accent)">{run.summary.failures} failure patterns</Tag>
              <Tag color="var(--blue)">{run.summary.hotspots} hotspots</Tag>
            </div>
          </div>

          <div style={{ ...S.panel, display: "grid", gap: 10 }}>
            <div style={{ display: "flex", justifyContent: "space-between", gap: 8, alignItems: "center", flexWrap: "wrap" }}>
              <div style={{ fontSize: 16, fontWeight: 700 }}>Prompt pack preview</div>
              <Btn
                onClick={() => copyText(run.prompt_pack)}
                style={{ padding: "4px 10px" }}
              >
                Copy
              </Btn>
            </div>
            <pre style={{
              margin: 0,
              whiteSpace: "pre-wrap",
              lineHeight: 1.6,
              color: "var(--text-dim)",
              background: "var(--bg-input)",
              border: "1px solid var(--border-in)",
              borderRadius: 8,
              padding: 12,
              maxHeight: 360,
              overflow: "auto",
            }}>
              {run.prompt_pack}
            </pre>
          </div>
        </div>
      ) : (
        <EmptyState icon="🧠" text="Run an ingest to turn merged PR history into reusable repo memory." />
      )}
    </div>
  );
}
