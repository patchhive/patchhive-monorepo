import { useEffect, useState } from "react";
import { createApiFetcher } from "@patchhivehq/product-shell";
import { API } from "../config.js";
import { Btn, EmptyState, Input, S, Tag, timeAgo } from "@patchhivehq/ui";

function statusColor(status) {
  if (status === "clear" || status === "resolved") {
    return "var(--green)";
  }
  if (status === "attention") {
    return "var(--accent)";
  }
  return "var(--gold)";
}

function itemStatusColor(status) {
  if (status === "resolved") {
    return "var(--green)";
  }
  if (status === "mixed") {
    return "var(--gold)";
  }
  return "var(--accent)";
}

export default function ReviewPanel({
  apiKey,
  form,
  setForm,
  running,
  onRun,
  review,
  onLoadReview,
}) {
  const [overview, setOverview] = useState(null);
  const fetch_ = createApiFetcher(apiKey);

  useEffect(() => {
    fetch_(`${API}/overview`)
      .then((res) => res.json())
      .then(setOverview)
      .catch(() => setOverview(null));
  }, [apiKey, review?.id]);

  async function copyPrompts() {
    if (!review?.prompt_suggestions?.length || !navigator?.clipboard) {
      return;
    }
    await navigator.clipboard.writeText(review.prompt_suggestions.join("\n"));
  }

  return (
    <div style={{ display: "grid", gap: 16 }}>
      <div style={{ ...S.panel, display: "grid", gap: 14 }}>
        <div style={{ display: "flex", justifyContent: "space-between", gap: 12, flexWrap: "wrap", alignItems: "center" }}>
          <div>
            <div style={{ fontSize: 18, fontWeight: 700 }}>Review a GitHub PR</div>
            <div style={{ color: "var(--text-dim)", fontSize: 12 }}>
              ReviewBee reads review threads, keeps the actionable bits, and turns them into a concrete checklist.
            </div>
          </div>
          <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
            <Tag color="var(--gold)">review threads</Tag>
            <Tag color="var(--blue)">actionable only</Tag>
            <Tag color="var(--accent)">merge checklist</Tag>
          </div>
        </div>

        <div style={{ display: "grid", gridTemplateColumns: "minmax(260px, 2fr) minmax(140px, 1fr) auto", gap: 12, alignItems: "end" }}>
          <div>
            <div style={S.label}>Repository</div>
            <Input value={form.repo} onChange={(value) => setForm((prev) => ({ ...prev, repo: value }))} placeholder="owner/repo" />
          </div>
          <div>
            <div style={S.label}>PR Number</div>
            <Input value={form.pr_number} onChange={(value) => setForm((prev) => ({ ...prev, pr_number: value }))} placeholder="123" />
          </div>
          <Btn onClick={onRun} disabled={running}>
            {running ? "Reading reviews..." : "Run ReviewBee"}
          </Btn>
        </div>
      </div>

      {review ? (
        <div style={{ display: "grid", gap: 16 }}>
          <div style={{ ...S.panel, display: "grid", gap: 12 }}>
            <div style={{ display: "flex", justifyContent: "space-between", gap: 12, flexWrap: "wrap", alignItems: "start" }}>
              <div style={{ display: "grid", gap: 8 }}>
                <div style={{ fontSize: 18, fontWeight: 700 }}>{review.pr_title}</div>
                <div style={{ color: "var(--text-dim)", fontSize: 12, lineHeight: 1.6 }}>{review.summary}</div>
                <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
                  <Tag color="var(--blue)">{review.repo}</Tag>
                  <Tag color="var(--blue)">PR #{review.pr_number}</Tag>
                  <Tag color={statusColor(review.status)}>{review.status}</Tag>
                  <Tag color="var(--text-dim)">{timeAgo(review.created_at)}</Tag>
                </div>
              </div>
              <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
                {review.pr_url && (
                  <Btn
                    onClick={() => window.open(review.pr_url, "_blank", "noreferrer")}
                    style={{ padding: "6px 10px" }}
                  >
                    Open PR
                  </Btn>
                )}
                {review.prompt_suggestions?.length > 0 && (
                  <Btn onClick={copyPrompts} style={{ padding: "6px 10px" }}>
                    Copy Prompts
                  </Btn>
                )}
              </div>
            </div>

            <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(160px, 1fr))", gap: 10 }}>
              <div style={{ ...S.panel, padding: 12, display: "grid", gap: 4 }}>
                <div style={S.label}>Open items</div>
                <div style={{ fontSize: 22, fontWeight: 700, color: "var(--accent)" }}>{review.metrics.open_items}</div>
              </div>
              <div style={{ ...S.panel, padding: 12, display: "grid", gap: 4 }}>
                <div style={S.label}>Resolved items</div>
                <div style={{ fontSize: 22, fontWeight: 700, color: "var(--green)" }}>{review.metrics.resolved_items}</div>
              </div>
              <div style={{ ...S.panel, padding: 12, display: "grid", gap: 4 }}>
                <div style={S.label}>Actionable threads</div>
                <div style={{ fontSize: 22, fontWeight: 700 }}>{review.metrics.actionable_threads}</div>
              </div>
              <div style={{ ...S.panel, padding: 12, display: "grid", gap: 4 }}>
                <div style={S.label}>Reviewers</div>
                <div style={{ fontSize: 22, fontWeight: 700 }}>{review.metrics.reviewer_count}</div>
              </div>
            </div>

            <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
              <Tag color="var(--accent)">{review.metrics.requested_changes_reviews} requested changes</Tag>
              <Tag color="var(--green)">{review.metrics.approval_reviews} approvals</Tag>
              <Tag color="var(--gold)">{review.metrics.comment_reviews} comment reviews</Tag>
              <Tag color="var(--blue)">{review.metrics.thread_count} total threads</Tag>
            </div>

            {review.reviewers?.length > 0 && (
              <div style={{ color: "var(--text-dim)", fontSize: 12 }}>
                Reviewers in play: {review.reviewers.join(", ")}
              </div>
            )}
          </div>

          {review.prompt_suggestions?.length > 0 && (
            <div style={{ ...S.panel, display: "grid", gap: 10 }}>
              <div style={{ fontSize: 15, fontWeight: 700 }}>Suggested follow-up prompts</div>
              {review.prompt_suggestions.map((prompt, index) => (
                <div key={`${prompt}-${index}`} style={{ border: "1px solid var(--border)", borderRadius: 6, padding: 12, background: "var(--bg-input)", color: "var(--text-dim)", lineHeight: 1.6 }}>
                  {prompt}
                </div>
              ))}
            </div>
          )}

          <div style={{ display: "grid", gap: 12 }}>
            {review.checklist.length === 0 ? (
              <EmptyState icon="✓" text="ReviewBee did not find actionable checklist items for this PR." />
            ) : (
              review.checklist.map((item) => (
                <div key={item.key} style={{ ...S.panel, display: "grid", gap: 10 }}>
                  <div style={{ display: "flex", justifyContent: "space-between", gap: 12, flexWrap: "wrap", alignItems: "start" }}>
                    <div style={{ display: "grid", gap: 6 }}>
                      <div style={{ fontSize: 16, fontWeight: 700 }}>{item.title}</div>
                      <div style={{ color: "var(--text-dim)", fontSize: 12, lineHeight: 1.6 }}>{item.summary}</div>
                    </div>
                    <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
                      <Tag color={itemStatusColor(item.status)}>{item.status}</Tag>
                      <Tag color="var(--blue)">{item.category}</Tag>
                      <Tag color="var(--accent)">{item.open_threads} open</Tag>
                      <Tag color="var(--green)">{item.resolved_threads} resolved</Tag>
                    </div>
                  </div>

                  <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
                    {item.path_hints.map((path) => (
                      <Tag key={path} color="var(--blue)">{path}</Tag>
                    ))}
                    {item.commenter_logins.map((login) => (
                      <Tag key={login} color="var(--text-dim)">@{login}</Tag>
                    ))}
                  </div>

                  <div style={{ border: "1px solid var(--border)", borderRadius: 6, padding: 12, background: "var(--bg-input)", color: "var(--text-dim)", lineHeight: 1.6 }}>
                    {item.prompt_hint}
                  </div>

                  {item.evidence.length > 0 && (
                    <div style={{ display: "grid", gap: 8 }}>
                      {item.evidence.map((evidence, index) => (
                        <div key={`${evidence.url}-${index}`} style={{ borderTop: "1px solid var(--border)", paddingTop: 8, display: "grid", gap: 6 }}>
                          <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
                            {evidence.author_login && <Tag color="var(--gold)">@{evidence.author_login}</Tag>}
                            {evidence.path && <Tag color="var(--blue)">{evidence.path}</Tag>}
                            {evidence.resolved && <Tag color="var(--green)">resolved</Tag>}
                            {evidence.outdated && <Tag color="var(--gold)">outdated</Tag>}
                          </div>
                          <div style={{ color: "var(--text-dim)", fontSize: 12, lineHeight: 1.6 }}>{evidence.excerpt}</div>
                          {evidence.url && (
                            <a href={evidence.url} target="_blank" rel="noreferrer" style={{ fontSize: 11, color: "var(--accent)", textDecoration: "none" }}>
                              Open comment ↗
                            </a>
                          )}
                        </div>
                      ))}
                    </div>
                  )}
                </div>
              ))
            )}
          </div>
        </div>
      ) : overview ? (
        <div style={{ display: "grid", gap: 16 }}>
          <div style={{ ...S.panel, display: "grid", gap: 12 }}>
            <div style={{ fontSize: 18, fontWeight: 700 }}>{overview.product}</div>
            <div style={{ color: "var(--accent)", fontSize: 12 }}>{overview.tagline}</div>
            <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(160px, 1fr))", gap: 10 }}>
              <div style={{ ...S.panel, padding: 12, display: "grid", gap: 4 }}>
                <div style={S.label}>Stored reviews</div>
                <div style={{ fontSize: 22, fontWeight: 700 }}>{overview.counts.reviews}</div>
              </div>
              <div style={{ ...S.panel, padding: 12, display: "grid", gap: 4 }}>
                <div style={S.label}>Repos seen</div>
                <div style={{ fontSize: 22, fontWeight: 700 }}>{overview.counts.repos}</div>
              </div>
              <div style={{ ...S.panel, padding: 12, display: "grid", gap: 4 }}>
                <div style={S.label}>Open review items</div>
                <div style={{ fontSize: 22, fontWeight: 700, color: "var(--accent)" }}>{overview.counts.open_items}</div>
              </div>
            </div>
          </div>

          <div style={{ ...S.panel, display: "grid", gap: 12 }}>
            <div style={{ fontSize: 15, fontWeight: 700 }}>Recent ReviewBee runs</div>
            {overview.recent_reviews?.length ? (
              overview.recent_reviews.map((item) => (
                <div key={item.id} style={{ display: "flex", justifyContent: "space-between", gap: 12, flexWrap: "wrap", alignItems: "center", borderTop: "1px solid var(--border)", paddingTop: 10 }}>
                  <div style={{ display: "grid", gap: 4 }}>
                    <div style={{ fontWeight: 700 }}>{item.repo} · PR #{item.pr_number}</div>
                    <div style={{ color: "var(--text-dim)", fontSize: 12 }}>{item.summary}</div>
                    <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
                      <Tag color={statusColor(item.status)}>{item.status}</Tag>
                      <Tag color="var(--accent)">{item.open_items} open</Tag>
                      <Tag color="var(--green)">{item.resolved_items} resolved</Tag>
                      <Tag color="var(--text-dim)">{timeAgo(item.created_at)}</Tag>
                    </div>
                  </div>
                  <Btn onClick={() => onLoadReview(item.id)}>Load</Btn>
                </div>
              ))
            ) : (
              <EmptyState icon="🐝" text="Run ReviewBee on a PR and your recent checklists will show up here." />
            )}
          </div>
        </div>
      ) : (
        <EmptyState icon="?" text="ReviewBee overview data is not available yet." />
      )}
    </div>
  );
}
