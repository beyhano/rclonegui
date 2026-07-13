interface Props {
  value: string;
  onChange: (expr: string) => void;
}

const PRESETS = [
  { label: "Every hour", value: "0 0 * * * *" },
  { label: "Every 6 hours", value: "0 0 */6 * * *" },
  { label: "Daily at midnight", value: "0 0 0 * * *" },
  { label: "Daily at 03:00", value: "0 0 3 * * *" },
  { label: "Weekly (Mon 03:00)", value: "0 0 3 * * 1" },
  { label: "Monthly (1st 03:00)", value: "0 0 3 1 * *" },
];

export default function CronInput({ value, onChange }: Props) {
  return (
    <div className="cron-input">
      <label>Schedule (cron expression)</label>
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
