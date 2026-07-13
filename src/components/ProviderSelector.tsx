import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface Provider {
  Name: string;
  Description: string;
  Prefix: string;
}

interface Props {
  value: string;
  onChange: (prefix: string) => void;
  label: string;
}

export default function ProviderSelector({ value, onChange, label }: Props) {
  const [providers, setProviders] = useState<Provider[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    invoke<Provider[]>("rclone_providers")
      .then(setProviders)
      .catch(console.error)
      .finally(() => setLoading(false));
  }, []);

  return (
    <div className="provider-selector">
      <label>{label}</label>
      {loading ? (
        <select disabled><option>Loading providers...</option></select>
      ) : (
        <select value={value} onChange={e => onChange(e.target.value)}>
          <option value="">-- Select provider --</option>
          {providers.map(p => (
            <option key={p.Prefix} value={p.Prefix}>
              {p.Name} — {p.Description}
            </option>
          ))}
        </select>
      )}
    </div>
  );
}
