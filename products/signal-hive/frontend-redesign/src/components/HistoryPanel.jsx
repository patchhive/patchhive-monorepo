import { useState, useEffect } from "react";
import { API } from "../config.js";
import { timeAgo } from "../report.js";

function createApiFetcher(key) {
  return (url, opts = {}) => fetch(url, {
    ...opts,
    headers: { ...opts.headers, "X-API-Key": key },
  });
}

export default function HistoryPanel({ apiKey, scans, loadScan, downloadReport }) {
  const fetch_ = createApiFetcher(apiKey);
  const [timelines, setTimelines] = useState({});
  const [page, setPage] = useState(0);
  const PAGE_SIZE = 20;

  useEffect(() => {
    scans.forEach(s => {
      if (!timelines[s.id]) {
        fetch_(`${API}/history/${s.id}/timeline`)
          .then(r => r.json())
          .then(data => setTimelines(prev => ({ ...prev, [s.id]: data })))
          .catch(() => {});
      }
    });
  }, [scans]);

  const visible = scans.slice(page * PAGE_SIZE, (page + 1) * PAGE_SIZE);

  return (
    <div style={{ display: "grid", gap: 12 }}>
      {scans.length === 0 && (
        <div className="empty-state">
          <div className="emoji">◎</div>
          <h3>No scans yet</h3>
          <p>Run your first scan from the Scan tab and it'll appear here.</p>
        </div>
      )}

      <div className="history-list">
        {visible.map(s => (
          <div key={s.id} className="history-item">
            <div style={{ display: "grid", gap: 4, minWidth: 0 }}>
              <div style={{ fontSize: 13, fontWeight: 600 }}>Scan #{s.id}</div>
              <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
                <span className="tag tag-surface">{timeAgo(s.created_at)}</span>
                {s.status && <span className={`tag ${s.status === "completed" ? "tag-green" : "tag-amber"}`}>{s.status}</span>}
                <span className="tag tag-surface">{s.repo_count || "—"} repos</span>
                {(s.stale_count || s.todo_count) && (
                  <>
                    {s.stale_count > 0 && <span className="tag tag-red">{s.stale_count} stale</span>}
                    {s.todo_count > 0 && <span className="tag tag-accent">{s.todo_count} TODOs</span>}
                  </>
                )}
              </div>
              {s.error && <div style={{ fontSize: 11, color: "var(--red)" }}>{s.error}</div>}
            </div>
            <div className="btn-group">
              <button className="btn btn-sm" onClick={() => loadScan(s.id)}>View</button>
              <button className="btn btn-sm" onClick={() => downloadReport(s.id)}>Export</button>
            </div>
          </div>
        ))}
      </div>

      {scans.length > PAGE_SIZE && (
        <div style={{ display: "flex", justifyContent: "center", gap: 8, marginTop: 8 }}>
          <button className="btn btn-sm" disabled={page === 0} onClick={() => setPage(p => p - 1)}>← Previous</button>
          <span style={{ fontSize: 12, color: "var(--text-muted)", alignSelf: "center" }}>Page {page + 1} of {Math.ceil(scans.length / PAGE_SIZE)}</span>
          <button className="btn btn-sm" disabled={(page + 1) * PAGE_SIZE >= scans.length} onClick={() => setPage(p => p + 1)}>Next →</button>
        </div>
      )}
    </div>
  );
}
