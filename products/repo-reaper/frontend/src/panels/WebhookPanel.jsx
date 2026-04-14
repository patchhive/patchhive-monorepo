import { S, Btn } from "@patchhivehq/ui";
import { API } from "../config.js";

export function WebhookPanel({ watchMode, onToggleWatch }) {
  const url = `${API.replace(/\/+$/, "")}/webhook/github`;
  const webhookRows = [
    ["Payload URL", url],
    ["Content type", "application/json"],
    ["Events", "Issues, Issue comments"],
  ];

  return (
    <div style={{ maxWidth: 600 }}>
      <div style={{ fontSize: 13, fontWeight: 700, color: "#d4d4e8", marginBottom: 16 }}>⚡ Webhooks & Watch Mode</div>

      <div style={{ ...S.panel, marginBottom: 16 }}>
        <div style={{ fontSize: 11, fontWeight: 700, color: "#d4d4e8", marginBottom: 10 }}>Watch Mode</div>
        <div style={{ fontSize: 10, color: "#484868", marginBottom: 12 }}>
          When enabled, newly labeled "bug" issues automatically trigger a hunt. Requires the GitHub webhook below to be configured.
        </div>
        <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
          <div style={{ fontSize: 13, color: watchMode ? "#2a8a4a" : "#484868" }}>
            {watchMode ? "● Active" : "○ Inactive"}
          </div>
          <Btn onClick={onToggleWatch} color={watchMode ? "#484868" : "#2a8a4a"}>
            {watchMode ? "Disable Watch Mode" : "Enable Watch Mode"}
          </Btn>
        </div>
      </div>

      <div style={S.panel}>
        <div style={{ fontSize: 11, fontWeight: 700, color: "#d4d4e8", marginBottom: 10 }}>GitHub Webhook Setup</div>
        <div style={{ fontSize: 10, color: "#484868", marginBottom: 12 }}>Configure this in your GitHub repo → Settings → Webhooks.</div>
        <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
          {webhookRows.map(([label, value]) => (
            <div key={label} style={{ display: "flex", gap: 8, alignItems: "center" }}>
              <span style={{ fontSize: 10, color: "#484868", minWidth: 100 }}>{label}</span>
              <code style={{ fontSize: 10, color: "#d4d4e8", background: "#10101e", padding: "3px 8px", borderRadius: 3, flex: 1 }}>
                {value}
              </code>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
