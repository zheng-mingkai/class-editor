interface TitleBarProps {
  mode: "system" | "light" | "dark";
  onThemeChange: (m: "system" | "light" | "dark") => void;
}

export function TitleBar({ mode, onThemeChange }: TitleBarProps) {
  const options: Array<"system" | "light" | "dark"> = ["system", "light", "dark"];
  const labels: Record<string, string> = {
    system: "跟随系统",
    light: "浅色",
    dark: "深色",
  };
  return (
    <div className="titlebar">
      <div className="titlebar-title">
        <span className="titlebar-brand-dot" />
        编辑class
      </div>
      <div className="theme-toggle">
        {options.map((o) => (
          <button
            key={o}
            className={mode === o ? "active" : ""}
            onClick={() => onThemeChange(o)}
          >
            {labels[o]}
          </button>
        ))}
      </div>
    </div>
  );
}
