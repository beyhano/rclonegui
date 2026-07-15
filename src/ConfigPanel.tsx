import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import Swal from "sweetalert2";
import type { Remote } from "./types";
import ConfigFormModal from "./components/ConfigFormModal";

interface EditRemoteData {
  name: string;
  provider: string;
  config: Record<string, string>;
}

function ConfigPanel() {
  const [remotes, setRemotes] = useState<Remote[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [showForm, setShowForm] = useState(false);
  const [editRemote, setEditRemote] = useState<EditRemoteData | undefined>(undefined);

  const loadRemotes = () => {
    setLoading(true);
    invoke<Remote[]>("rclone_config_list")
      .then(setRemotes)
      .catch(setError)
      .finally(() => setLoading(false));
  };

  useEffect(() => { loadRemotes(); }, []);

  const handleDelete = async (name: string) => {
    const result = await Swal.fire({
      title: "Uzak Sunucuyu Sil",
      text: `"${name}" uzak sunucusunu silmek istediğinize emin misiniz?`,
      icon: "warning",
      showCancelButton: true,
      confirmButtonText: "Evet, sil",
      cancelButtonText: "İptal",
      reverseButtons: true,
    });
    if (!result.isConfirmed) return;
    try {
      await invoke("rclone_config_delete", { name });
      loadRemotes();
    } catch (e) {
      Swal.fire("Hata", String(e), "error");
    }
  };

  const handleEdit = async (name: string) => {
    try {
      const result = await invoke<[string, Record<string, string>]>("rclone_config_get", { name });
      setEditRemote({ name, provider: result[0], config: result[1] });
      setShowForm(true);
    } catch (e) {
      Swal.fire("Hata", String(e), "error");
    }
  };

  if (loading) return <div className="panel-loading">Uzak sunucular yükleniyor…</div>;
  if (error) return <div className="panel-error">Hata: {error}</div>;

  return (
    <div className="config-panel">
      <div className="scheduler-header">
        <h2>Uzak Sunucular</h2>
        <button className="btn-primary" onClick={() => { setEditRemote(undefined); setShowForm(true); }}>+ Uzak Sunucu Ekle</button>
      </div>
      {remotes.length === 0 ? (
        <div className="empty-state">
          <p>Hiçbir uzak sunucu yapılandırılmamış.</p>
          <p>"Uzak Sunucu Ekle" butonuna tıklayarak bir tane oluşturun.</p>
        </div>
      ) : (
        <div className="remote-list">
          {remotes.map(r => (
            <div key={r.name} className="remote-card">
              <span className="remote-name">{r.name}</span>
              <div style={{ display: "flex", alignItems: "center", gap: "0.5rem" }}>
                <span className="badge">{r.type}</span>
                <button
                  onClick={() => handleEdit(r.name)}
                  style={{ background: "none", border: "1px solid #ccc", borderRadius: 4, padding: "0.2rem 0.5rem", cursor: "pointer", fontSize: "0.8rem", boxShadow: "none" }}
                  title="Düzenle"
                >
                  ✏️ Düzenle
                </button>
                <button
                  onClick={() => handleDelete(r.name)}
                  style={{ background: "none", border: "1px solid #ccc", borderRadius: 4, padding: "0.2rem 0.5rem", cursor: "pointer", fontSize: "0.8rem", color: "#c62828", boxShadow: "none" }}
                  title="Sil"
                >
                  🗑️ Sil
                </button>
              </div>
            </div>
          ))}
        </div>
      )}
      {showForm && (
        <ConfigFormModal
          editRemote={editRemote}
          onClose={() => { setShowForm(false); setEditRemote(undefined); }}
          onCreated={loadRemotes}
        />
      )}
    </div>
  );
}

export default ConfigPanel;
