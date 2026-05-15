import { useState, useMemo } from "react";
import { API } from "../config.js";
import { sortRepos, SORT_OPTIONS } from "../sort.js";
import { summarizeScanHighlights } from "../report.js";
import SignalCard from "./SignalCard.jsx";

function toList(value) {
  return value.split(",").map(p => p.trim()).filter(Boolean);
}

function toRequestParams(params) {
  return {
    search_query: params.search_query,
    topics: toList(params.topics),
    languages: toList(params.languages),
    min_stars: Number(params.min_stars) || 25,
    max_repos: Number(params.max_repos) || 8,
    issues_per_repo: Number(params.issues_per_repo) || 30,
    stale_days: Number(params.stale_days) || 45,
  };
}

export default function ScanForm({ apiKey, params, setParams, running, onRun, scan, setScan, loadScan, downloadReport }) {
  const [sortBy, setSortBy] = useState("priority");
  const [expandedRepos, setExpandedRepos] = useState(new Set());
  const sortedRepos = useMemo(() => sortRepos(scan?.repos || [], sortBy), [scan, sortBy]);
  const highlights = useMemo(() => summarizeScanHighlights(scan), [scan]);

  const set = (key, value) => setParams(prev => ({ ...prev, [key]: value }));

  const toggleRepo = (name) => {
    setExpandedRepos(prev => {
      const next = new Set(prev);
      if (next.has(name)) next.delete(name);
      else next.add(name);
      return next;
    });
  };

  const handleSubmit = (e) => {
    e.preventDefault();
    onRun(toRequestParams(params));
  };

  return (
    <div style={{ display: "grid", gap: 16 }}>
      {/* Scan Form */}
      <div className="card">
        <div className="card-header">
          <h3>Scan Configuration</h3>
          <div className="btn-group">
            {scan && (
              <>
                <button className="btn btn-sm" onClick={() => downloadReport(scan.id)}>Export Report</button>
                <button className="btn btn-sm" onClick={() => { navigator.clipboard.writeText(buildDashboardSummary(scan)); }}>Copy Summary</button>
              </>
            )}
          </div>
        </div>
        <div className="card-body">
          <form onSubmit={handleSubmit} style={{ display: "grid", gap: 16 }}>
            <div className="scan-form">
              <div className="field">
                <label>Search Query</label>
                <input value={params.search_query} onChange={e => set("search_query", e.target.value)} placeholder="bug triage, maintenance, backlog" />
              </div>
              <div className="field">
                <label>Topics</label>
                <input value={params.topics} onChange={e => set("topics", e.target.value)} placeholder="payments, api, maintenance" />
              </div>
              <div className="field">
                <label>Languages</label>
                <input value={params.languages} onChange={e => set("languages", e.target.value)} placeholder="rust,typescript,python" />
              </div>
              <div className="field">
                <label>Min Stars</label>
                <input value={params.min_stars} onChange={e => set("min_stars", e.target.value)} placeholder="25" type="number" />
              </div>
              <div className="field">
                <label>Max Repos</label>
                <input value={params.max_repos} onChange={e => set("max_repos", e.target.value)} placeholder="8" type="number" />
              </div>
              <div className="field">
                <label>Issues / Repo</label>
                <input value={params.issues_per_repo} onChange={e => set("issues_per_repo", e.target.value)} placeholder="30" type="number" />
              </div>
              <div className="field">
                <label>Stale Threshold</label>
                <input value={params.stale_days} onChange={e => set("stale_days", e.target.value)} placeholder="45" type="number" />
              </div>
            </div>
            <div style={{ display: "flex", gap: 10, alignItems: "center" }}>
              <button type="submit" className="btn btn-primary" disabled={running}>
                {running ? "Scanning…" : "Run Scan"}
              </button>
              {running && <span style={{ fontSize: 12, color: "var(--text-muted)" }}>Scanning repositories for maintenance signals…</span>}
            </div>
          </form>
        </div>
      </div>

      {/* Results */}
      {scan && (
        <div className="card">
          <div className="card-header">
            <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
              <h3>Results</h3>
              <span className="tag tag-accent">{scan.repos?.length || 0} repos</span>
              {highlights.map((h, i) => (
                <span key={i} className="tag tag-surface">{h}</span>
              ))}
            </div>
            <div className="field" style={{ flexDirection: "row", alignItems: "center", gap: 8 }}>
              <label style={{ whiteSpace: "nowrap" }}>Sort</label>
              <select value={sortBy} onChange={e => setSortBy(e.target.value)} style={{ padding: "6px 10px", borderRadius: "var(--radius-sm)", border: "1px solid var(--border)", background: "var(--surface)", color: "var(--text)", fontSize: 12, fontFamily: "inherit" }}>
                {SORT_OPTIONS.map(o => <option key={o.v} value={o.v}>{o.l}</option>)}
              </select>
            </div>
          </div>
          <div className="card-body">
            <div className="repo-grid">
              {sortedRepos.length === 0 && (
                <div className="empty-state">
                  <div className="emoji">🔍</div>
                  <h3>No signals found</h3>
                  <p>Try broadening your search — wider topics, lower star threshold, or more repos.</p>
                </div>
              )}
              {sortedRepos.map(repo => (
                <SignalCard
                  key={repo.full_name}
                  repo={repo}
                  expanded={expandedRepos.has(repo.full_name)}
                  onToggle={() => toggleRepo(repo.full_name)}
                />
              ))}
            </div>
          </div>
        </div>
      )}

      {/* No scan yet */}
      {!scan && (
        <div className="empty-state">
          <div className="emoji">🔍</div>
          <h3>Ready to scan</h3>
          <p>Configure your scan parameters above and click <strong>Run Scan</strong> to discover maintenance signals across repositories.</p>
        </div>
      )}
    </div>
  );
}
