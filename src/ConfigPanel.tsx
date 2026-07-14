import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { Remote } from "./types";
import ConfigFormModal from "./components/ConfigFormModal";

function ConfigPanel() {
  const [remotes, setRemotes] = useState<Remote[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [showForm, setShowForm] = useState(false);

  const loadRemotes = () => {
    setLoading(true);
    invoke<Remote[]>("rclone_config_list")
      .then(setRemotes)
      .catch(setError)
      .finally(() => setLoading(false));
  };

  useEffect(() => { loadRemotes(); }, []);

  if (loading) return <div className="panel-loading">Loading remotes…</div>;
  if (error) return <div className="panel-error">Error: {error}</div>;

  return (
    <div className="config-panel">
      <div className="scheduler-header">
        <h2>Configured Remotes</h2>
        <button className="btn-primary" onClick={() => setShowForm(true)}>+ Add Remote</button>
      </div>
      {remotes.length === 0 ? (
        <div className="empty-state">
          <p>No remotes configured.</p>
          <p>Click "Add Remote" to create one.</p>
        </div>
      ) : (
        <div className="remote-list">
          {remotes.map(r => (
            <div key={r.name} className="remote-card">
              <span className="remote-name">{r.name}</span>
              <span className="badge">{r.type}</span>
            </div>
          ))}
        </div>
      )}
      {showForm && <ConfigFormModal onClose={() => setShowForm(false)} onCreated={loadRemotes} />}
    </div>
  );
}

export default ConfigPanel;
