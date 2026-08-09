interface ToolbarProps {
  onOpen: () => void;
  onSave: () => void;
  saved: boolean;
  loading: boolean;
  hasFile: boolean;
  onOpenSettings: () => void;
}

export function Toolbar({
  onOpen,
  onSave,
  saved,
  loading,
  hasFile,
  onOpenSettings,
}: ToolbarProps) {
  return (
    <div className="toolbar">
      <button className="btn" onClick={onOpen} disabled={loading}>
        打开文件
      </button>
      <button
        className="btn btn-primary"
        onClick={onSave}
        disabled={!hasFile || saved || loading}
      >
        保存修改
      </button>
      <div className="toolbar-spacer" />
      {loading && <span className="file-name">处理中…</span>}
      <button className="btn btn-ghost" onClick={onOpenSettings}>
        设置
      </button>
    </div>
  );
}
