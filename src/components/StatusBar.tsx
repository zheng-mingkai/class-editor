interface StatusBarProps {
  saved: boolean;
  modifiedCount: number;
  jarSigned: boolean;
  error: string | null;
}

export function StatusBar({
  saved,
  modifiedCount,
  jarSigned,
  error,
}: StatusBarProps) {
  return (
    <div className="statusbar">
      <div className="status-item">
        <span
          className={`status-dot ${saved ? "ok" : modifiedCount > 0 ? "warn" : ""}`}
        />
        {saved ? "已保存" : `${modifiedCount} 项未保存`}
      </div>
      {jarSigned && (
        <div className="status-item">
          <span className="status-dot warn" />
          JAR 已签名
        </div>
      )}
      <div className="status-spacer" />
      {error && <span className="status-error">{error}</span>}
      <div className="status-item">编辑class v0.1.0</div>
    </div>
  );
}
