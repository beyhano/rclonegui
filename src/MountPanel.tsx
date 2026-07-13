import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { MountInfo } from "./types";

function MountPanel() {
  const [remote, setRemote] = useState("");
  const [mountPoint, setMountPoint] = useState("");
  const [mounts, setMounts] = useState<MountInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  async function fetchMounts() {
    try {
      const data = await invoke<MountInfo[]>("rclone_mount_list");
      setMounts(data);
    } catch (err: unknown) {
      setError(String(err));
    }
  }

  useEffect(() => {
    fetchMounts().then(() => setLoading(false));

    const unlisteners: Array<() => void> = [];

    async function setup() {
      const u1 = await listen("rclone:mount-status", () => {
        fetchMounts();
      });
      unlisteners.push(u1);
    }

    setup();

    return () => {
      unlisteners.forEach((u) => u());
    };
  }, []);

  async function handleMount() {
    if (!remote || !mountPoint) return;
    setError(null);
    try {
      await invoke("rclone_mount", {
        remote,
        mount_point: mountPoint,
      });
      setRemote("");
      setMountPoint("");
      await fetchMounts();
    } catch (err: unknown) {
      setError(String(err));
    }
  }

  async function handleUnmount(id: string) {
    setError(null);
    try {
      await invoke("rclone_unmount", { mount_id: id });
      await fetchMounts();
    } catch (err: unknown) {
      setError(String(err));
    }
  }

  if (loading) return <div className="panel-loading">Loading mounts…</div>;

  return (
    <div className="mount-panel">
      <h2>Mount Management</h2>

      <div className="mount-inputs">
        <input
          placeholder="Remote name (e.g., gdrive)"
          value={remote}
          onChange={(e) => setRemote(e.currentTarget.value)}
        />
        <input
          placeholder="Mount point (e.g., /mnt/gdrive)"
          value={mountPoint}
          onChange={(e) => setMountPoint(e.currentTarget.value)}
        />
        <button onClick={handleMount} disabled={!remote || !mountPoint}>
          Mount
        </button>
      </div>

      {error && <div className="panel-error">{error}</div>}

      {mounts.length === 0 ? (
        <div className="panel-empty">No active mounts.</div>
      ) : (
        <div className="mount-list">
          {mounts.map((m) => (
            <div key={m.id} className="mount-card">
              <div className="mount-info">
                <span className="remote-name">{m.remote}</span>
                <span className="mount-path">{m.mount_point}</span>
                <span className={`badge badge-${m.status}`}>{m.status}</span>
              </div>
              <button
                onClick={() => handleUnmount(m.id)}
                className="btn-danger"
              >
                Unmount
              </button>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

export default MountPanel;
