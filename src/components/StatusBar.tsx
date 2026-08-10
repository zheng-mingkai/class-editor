import { invoke } from "@tauri-apps/api/core";

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
  const openGithub = () => {
    invoke("open_url", { url: "https://github.com/zheng-mingkai/class-editor" }).catch(() => {
      // 兜底：Web 模式下仍用 window.open
      window.open("https://github.com/zheng-mingkai/class-editor", "_blank", "noopener,noreferrer");
    });
  };

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
      <button className="status-link" onClick={openGithub} title="打开 GitHub 仓库">
        ⭐ GitHub
      </button>
      <div className="status-item">作者：mingkai</div>
      <div className="status-item">class编辑器 v0.2.0</div>
    </div>
  );
}
