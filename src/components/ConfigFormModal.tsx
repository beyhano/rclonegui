import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import ProviderSelector from "./ProviderSelector";
import ProviderConfigForm from "./ProviderConfigForm";
import { Provider } from "../types";

interface RemoteConfig {
  name: string;
  provider: string;
  config: Record<string, string>;
}

interface Props {
  onClose: () => void;
  onCreated: () => void;
  editRemote?: RemoteConfig;
}

export default function ConfigFormModal({ onClose, onCreated, editRemote }: Props) {
  const [step, setStep] = useState(editRemote ? 2 : 1);
  const [providers, setProviders] = useState<Provider[]>([]);
  const [name, setName] = useState(editRemote?.name ?? "");
  const [provider, setProvider] = useState(editRemote?.provider ?? "");
  const [config, setConfig] = useState<Record<string, string>>(editRemote?.config ?? {});
  const [error, setError] = useState("");
  const [submitting, setSubmitting] = useState(false);

  useEffect(() => {
    invoke<Provider[]>("rclone_providers").then(setProviders).catch(console.error);
  }, []);

  const getOptions = () => {
    const p = providers.find(p => p.Prefix === provider);
    return p?.Options ?? [];
  };

  const handleCreate = async () => {
    setSubmitting(true);
    setError("");
    try {
      await invoke("rclone_config_create", {
        name,
        provider,
        config: JSON.stringify(config),
      });
      onCreated();
      onClose();
    } catch (e) {
      setError(String(e));
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal" onClick={e => e.stopPropagation()}>
        <h2>{editRemote ? "Uzak Sunucu Düzenle" : "Uzak Sunucu Ekle"} — Adım {step}/2</h2>

        {step === 1 && !editRemote && (
          <div className="modal-step">
            <label>Sunucu Adı</label>
            <input
              value={name}
              onChange={e => setName(e.target.value)}
              autoFocus
              placeholder="ornek-remote"
            />
            <ProviderSelector
              label="Sağlayıcı"
              value={provider}
              onChange={setProvider}
            />
          </div>
        )}

        {step === 2 && (
          <div className="modal-step">
            <ProviderConfigForm
              options={getOptions()}
              values={config}
              onChange={(k, v) => setConfig(c => ({ ...c, [k]: v }))}
            />
          </div>
        )}

        {error && <p className="error">{error}</p>}

        <div className="modal-actions">
          {!editRemote && step > 1 && (
            <button onClick={() => setStep(s => s - 1)}>Geri</button>
          )}
          {!editRemote && step < 2 ? (
            <button
              onClick={() => setStep(s => s + 1)}
              disabled={!name || !provider}
            >
              İleri
            </button>
          ) : (
            <button onClick={handleCreate} disabled={submitting}>
              {submitting ? "Kaydediliyor..." : editRemote ? "Kaydet" : "Uzak Sunucu Ekle"}
            </button>
          )}
          <button onClick={onClose}>İptal</button>
        </div>
      </div>
    </div>
  );
}
