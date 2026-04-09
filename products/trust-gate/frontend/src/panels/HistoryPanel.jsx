import { useEffect, useState } from "react";
import { createApiFetcher } from "@patchhivehq/product-shell";
import { API } from "../config.js";
import { Btn, EmptyState, S, ScoreBadge, Tag, timeAgo } from "@patchhivehq/ui";

function recommendationColor(recommendation) {
  if (recommendation === "safe") {
    return "var(--green)";
  }
  if (recommendation === "warn") {
    return "var(--gold)";
  }
  return "var(--accent)";
}

function sourceLabel(review) {
  if (review.source_kind === "github_pr") {
    return review.pr_number ? `PR #${review.pr_number}` : "GitHub PR";
  }
  return "Manual";
}

export default function HistoryPanel({ apiKey, onLoadReview }) {
  const [reviews, setReviews] = useState([]);
  const [busyId, setBusyId] = useState("");
  const fetch_ = createApiFetcher(apiKey);

  const refresh = () =>
    fetch_(`${API}/history`)
      .then((res) => res.json())
      .then((data) => setReviews(data.reviews || []))
      .catch(() => setReviews([]));

  useEffect(() => {
    refresh();
  }, [apiKey]);

  return (
    <div style={{ display: "grid", gap: 18 }}>
      <div
        style={{
          ...S.panel,
          display: "flex",
          justifyContent: "space-between",
          gap: 12,
          alignItems: "center",
          flexWrap: "wrap",
        }}
      >
        <div>
          <div style={{ fontSize: 18, fontWeight: 700 }}>Review History</div>
          <div style={{ color: "var(--text-dim)", fontSize: 12 }}>
            TrustGate keeps prior diff decisions so you can see how your rules behave over time.
          </div>
        </div>
        <Btn onClick={refresh}>Refresh</Btn>
      </div>

      {reviews.length === 0 ? (
        <EmptyState icon="◎" text="TrustGate review history will show up here after you review your first diff." />
      ) : (
        reviews.map((review) => (
          <div key={review.id} style={{ ...S.panel, display: "grid", gap: 12 }}>
            <div
              style={{
                display: "flex",
                justifyContent: "space-between",
                gap: 12,
                flexWrap: "wrap",
                alignItems: "center",
              }}
            >
              <div style={{ display: "grid", gap: 4 }}>
                <div style={{ fontSize: 15, fontWeight: 700 }}>{review.repo}</div>
                <div style={{ color: "var(--text-dim)", fontSize: 12 }}>
                  {review.ai_source || "unknown"} · {timeAgo(review.created_at)}
                </div>
              </div>
              <div style={{ display: "flex", gap: 8, flexWrap: "wrap", alignItems: "center" }}>
                <Tag color="var(--blue)">{sourceLabel(review)}</Tag>
                <Tag color={recommendationColor(review.recommendation)}>{review.recommendation}</Tag>
                <ScoreBadge score={review.risk_score} />
              </div>
            </div>
            <div style={{ color: "var(--text-dim)", fontSize: 12, lineHeight: 1.5 }}>{review.summary}</div>
            <div style={{ display: "flex", gap: 8, flexWrap: "wrap", alignItems: "center" }}>
              <Tag color="var(--blue)">{review.files_changed} files</Tag>
              <Btn
                onClick={async () => {
                  setBusyId(review.id);
                  try {
                    await onLoadReview(review.id);
                  } finally {
                    setBusyId("");
                  }
                }}
                disabled={busyId === review.id}
              >
                {busyId === review.id ? "Loading..." : "Load Review"}
              </Btn>
            </div>
          </div>
        ))
      )}
    </div>
  );
}
