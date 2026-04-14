import { useEffect, useState } from "react";
import { useApiFetcher } from "@patchhivehq/product-shell";
import { S, Btn, EmptyState } from "@patchhivehq/ui";
import { API } from "../config.js";

export function SchedulesPanel({ apiKey = "" }) {
  const [schedules, setSchedules] = useState([]);
  const [form, setForm] = useState({ cron_expr: "nightly", config_json: "{}" });
  const fetch_ = useApiFetcher(apiKey);

  const load = () => fetch_(`${API}/schedules`).then(r => r.json()).then(d => setSchedules(d.schedules || []));

  useEffect(load, [fetch_]);

  const create = async () => {
    let cfg;
    try {
      cfg = JSON.parse(form.config_json);
    } catch {
      return;
    }
    await fetch_(`${API}/schedules`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ cron_expr: form.cron_expr, config_json: cfg }),
    });
    load();
  };

  const del = async id => {
    await fetch_(`${API}/schedules/${id}`, { method: "DELETE" });
    load();
  };

  const toggle = async id => {
    await fetch_(`${API}/schedules/${id}/toggle`, { method: "PATCH" });
    load();
  };

  return (
    <div>
      <div style={{ fontSize: 13, fontWeight: 700, color: "#d4d4e8", marginBottom: 16 }}>◎ Scheduled Hunts</div>
      <div style={{ ...S.panel, marginBottom: 16 }}>
        <div style={{ fontSize: 11, fontWeight: 700, color: "#d4d4e8", marginBottom: 10 }}>New Schedule</div>
        <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
          <div style={S.field}>
            <label style={S.label}>Frequency</label>
            <select value={form.cron_expr} onChange={e => setForm(f => ({ ...f, cron_expr: e.target.value }))} style={S.select}>
              <option value="hourly">Hourly</option>
              <option value="nightly">Nightly</option>
              <option value="weekly">Weekly</option>
            </select>
          </div>
          <div style={S.field}>
            <label style={S.label}>Run Config (JSON)</label>
            <textarea
              value={form.config_json}
              onChange={e => setForm(f => ({ ...f, config_json: e.target.value }))}
              style={{ ...S.input, height: 80, resize: "vertical", fontFamily: "monospace" }}
            />
          </div>
          <Btn onClick={create} color="#c41e3a" style={{ alignSelf: "flex-start" }}>Create Schedule</Btn>
        </div>
      </div>

      {schedules.length === 0 ? (
        <EmptyState icon="◌" text="No schedules. Create one above." />
      ) : (
        schedules.map(schedule => (
          <div key={schedule.id} style={{ ...S.panel, marginBottom: 8 }}>
            <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
              <span style={{ fontSize: 11, color: schedule.enabled ? "#2a8a4a" : "#484868" }}>
                {schedule.enabled ? "●" : "○"}
              </span>
              <span style={{ fontSize: 11, color: "#d4d4e8", fontWeight: 600 }}>{schedule.cron_expr}</span>
              <span style={{ fontSize: 10, color: "#484868", flex: 1 }}>next: {schedule.next_run?.slice(0, 16)}</span>
              <Btn onClick={() => toggle(schedule.id)} color={schedule.enabled ? "#484868" : "#2a8a4a"} style={{ fontSize: 10 }}>
                {schedule.enabled ? "Pause" : "Resume"}
              </Btn>
              <Btn onClick={() => del(schedule.id)} color="#c41e3a" style={{ fontSize: 10 }}>Delete</Btn>
            </div>
          </div>
        ))
      )}
    </div>
  );
}
