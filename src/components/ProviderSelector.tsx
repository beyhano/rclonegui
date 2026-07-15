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
  const [search, setSearch] = useState("");

  useEffect(() => {
    invoke<Provider[]>("rclone_providers")
      .then(setProviders)
      .catch(console.error)
      .finally(() => setLoading(false));
  }, []);

  const filtered = search.trim()
    ? providers.filter(p =>
        p.Name.toLowerCase().includes(search.toLowerCase()) ||
        p.Description.toLowerCase().includes(search.toLowerCase())
      )
    : providers;

  return (
    <div className="provider-selector">
      <label>{label}</label>
      {loading ? (
        <select disabled><option>Sağlayıcılar yükleniyor...</option></select>
      ) : (
        <div style={{ display: "flex", flexDirection: "column", gap: "0.35rem" }}>
          <input
            type="text"
            value={search}
            onChange={e => setSearch(e.target.value)}
            placeholder="Sağlayıcı ara... (örn: drive, s3, dropbox)"
            style={{ padding: "0.5rem", borderRadius: 6, border: "1px solid #ccc", fontSize: "0.9rem" }}
            autoFocus={!value}
          />
          <select
            value={value}
            onChange={e => onChange(e.target.value)}
            size={Math.min(filtered.length + 1, 10)}
            style={{ minHeight: 120 }}
          >
            <option value="">-- Sağlayıcı seçin --</option>
            {filtered.map(p => (
              <option key={p.Prefix} value={p.Prefix}>
                {p.Name} — {p.Description}
              </option>
            ))}
            {filtered.length === 0 && (
              <option disabled>Eşleşen sağlayıcı bulunamadı</option>
            )}
          </select>
        </div>
      )}
    </div>
  );
}
