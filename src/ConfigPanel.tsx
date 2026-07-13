import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { Remote } from "./types";

function ConfigPanel() {
  const [remotes, setRemotes] = useState<Remote[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    invoke<Remote[]>("rclone_config_list")
      .then((data) => {
        setRemotes(data);
        setLoading(false);
      })
      .catch((err: unknown) => {
        setError(String(err));
        setLoading(false);
      });
  }, []);

  if (loading) return <div className="panel-loading">Loading remotes…</div>;
  if (error) return <div className="panel-error">Error: {error}</div>;

  if (remotes.length === 0) {
    return (
      <div className="panel-empty">
        No remotes configured. Run <code>rclone config</code> to add one.
      </div>
    );
  }

  return (
    <div className="config-panel">
      <h2>Configured Remotes</h2>
      <div className="remote-list">
        {remotes.map((r) => (
          <div key={r.name} className="remote-card">
            <span className="remote-name">{r.name}</span>
            <span className="badge">{r.type}</span>
          </div>
        ))}
      </div>
    </div>
  );
}

export default ConfigPanel;
