interface Props {
  value: string;
  onChange: (expr: string) => void;
}

const PRESETS = [
  { label: "Her Saat", value: "0 0 * * * *" },
  { label: "Her 6 Saat", value: "0 0 */6 * * *" },
  { label: "Her Gece 00:00", value: "0 0 0 * * *" },
  { label: "Her Gece 03:00", value: "0 0 3 * * *" },
  { label: "Haftalık (Pzt 03:00)", value: "0 0 3 * * 1" },
  { label: "Aylık (1. gün 03:00)", value: "0 0 3 1 * *" },
];

export default function CronInput({ value, onChange }: Props) {
  return (
    <div className="cron-input">
      <label>Zamanlama (cron ifadesi)</label>
      <input
        type="text"
        value={value}
        onChange={e => onChange(e.target.value)}
        placeholder="0 3 * * * *"
      />
      <div className="cron-presets">
        {PRESETS.map(p => (
          <button
            key={p.value}
            type="button"
            className={`cron-preset ${value === p.value ? "active" : ""}`}
            onClick={() => onChange(p.value)}
          >
            {p.label}
          </button>
        ))}
      </div>
    </div>
  );
}
