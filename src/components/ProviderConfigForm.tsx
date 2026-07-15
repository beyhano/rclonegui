interface ProviderOption {
  Name: string;
  Help: string;
  Default: unknown;
  Required: boolean;
  Type: string;
  Advanced: boolean;
  IsPassword: boolean;
  Exclusive: boolean;
  Examples: Array<{ Value: string; Help: string }> | null;
}

interface Props {
  options: ProviderOption[];
  values: Record<string, string>;
  onChange: (name: string, value: string) => void;
}

export default function ProviderConfigForm({ options, values, onChange }: Props) {
  const basicOptions = options.filter(o => !o.Advanced);
  const advancedOptions = options.filter(o => o.Advanced);

  const renderField = (opt: ProviderOption) => {
    const value = values[opt.Name] ?? (opt.Default as string) ?? "";

    if (opt.Type === "bool") {
      return (
        <label key={opt.Name} className="config-field config-field--bool">
          <input
            type="checkbox"
            checked={value === "true"}
            onChange={e => onChange(opt.Name, e.target.checked ? "true" : "false")}
          />
          <span>{opt.Help}</span>
        </label>
      );
    }

    if (opt.Exclusive && opt.Examples && opt.Examples.length > 0) {
      return (
        <div key={opt.Name} className="config-field">
          <label>{opt.Help}{opt.Required && " *"}</label>
          <select value={value} onChange={e => onChange(opt.Name, e.target.value)}>
            <option value="">-- Seçin --</option>
            {opt.Examples.map(ex => (
              <option key={ex.Value} value={ex.Value}>{ex.Help} ({ex.Value})</option>
            ))}
          </select>
        </div>
      );
    }

    return (
      <div key={opt.Name} className="config-field">
        <label>{opt.Help}{opt.Required && " *"}</label>
        <input
          type={opt.IsPassword ? "password" : "text"}
          value={value}
          onChange={e => onChange(opt.Name, e.target.value)}
          placeholder={typeof opt.Default === "string" ? opt.Default : ""}
        />
      </div>
    );
  };

  if (options.length === 0) {
    return <p className="config-empty">Bir sağlayıcı seçin.</p>;
  }

  return (
    <div className="provider-config-form">
      {basicOptions.map(renderField)}
      {advancedOptions.length > 0 && (
        <details className="config-advanced">
          <summary>Gelişmiş Seçenekler ({advancedOptions.length})</summary>
          {advancedOptions.map(renderField)}
        </details>
      )}
    </div>
  );
}
