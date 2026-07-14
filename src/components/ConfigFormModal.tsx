import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import ProviderSelector from "./ProviderSelector";
import ProviderConfigForm from "./ProviderConfigForm";
import { Provider } from "../types";

interface Props {
  onClose: () => void;
  onCreated: () => void;
}

export default function ConfigFormModal({ onClose, onCreated }: Props) {
  const [step, setStep] = useState(1);
  const [providers, setProviders] = useState<Provider[]>([]);
  const [name, setName] = useState("");
  const [provider, setProvider] = useState("");
  const [config, setConfig] = useState<Record<string, string>>({});
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
        <h2>Add Remote — Step {step}/2</h2>

        {step === 1 && (
          <div className="modal-step">
            <label>Remote Name</label>
            <input
              value={name}
              onChange={e => setName(e.target.value)}
              autoFocus
              placeholder="my-remote"
            />
            <ProviderSelector
              label="Provider"
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
          {step > 1 && (
            <button onClick={() => setStep(s => s - 1)}>Back</button>
          )}
          {step < 2 ? (
            <button
              onClick={() => setStep(s => s + 1)}
              disabled={!name || !provider}
            >
              Next
            </button>
          ) : (
            <button onClick={handleCreate} disabled={submitting}>
              {submitting ? "Creating..." : "Add Remote"}
            </button>
          )}
          <button onClick={onClose}>Cancel</button>
        </div>
      </div>
    </div>
  );
}
