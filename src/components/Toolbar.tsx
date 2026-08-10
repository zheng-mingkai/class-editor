interface ToolbarProps {
  onOpen: () => void;
  onSave: () => void;
  saved: boolean;
  loading: boolean;
  hasFile: boolean;
  canUndo: boolean;
  canRedo: boolean;
  onUndo: () => void;
  onRedo: () => void;
  onOpenSettings: () => void;
  onOpenSearch?: () => void;
}

export function Toolbar({
  onOpen,
  onSave,
  saved,
  loading,
  hasFile,
  canUndo,
  canRedo,
  onUndo,
  onRedo,
  onOpenSettings,
  onOpenSearch,
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
      <div className="toolbar-divider" />
      <button
        className="btn btn-ghost"
        onClick={onUndo}
        disabled={!canUndo || loading}
        title="撤销 (Ctrl+Z)"
      >
        撤销
      </button>
      <button
        className="btn btn-ghost"
        onClick={onRedo}
        disabled={!canRedo || loading}
        title="重做 (Ctrl+Y)"
      >
        重做
      </button>
      {onOpenSearch && (
        <>
          <div className="toolbar-divider" />
          <button
            className="btn"
            onClick={onOpenSearch}
            disabled={loading}
            title="全局搜索替换"
          >
            全局搜索
          </button>
        </>
      )}
      <div className="toolbar-spacer" />
      {loading && <span className="file-name">处理中…</span>}
      <button className="btn btn-ghost" onClick={onOpenSettings}>
        设置
      </button>
    </div>
  );
}
