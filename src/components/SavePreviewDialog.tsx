import { useMemo } from "react";
import type { ClassFilePreview } from "../types/classfile";

interface SavePreviewDialogProps {
  modifications: Map<number, string>;
  preview: ClassFilePreview | null;
  onConfirm: () => void;
  onCancel: () => void;
}

interface DiffRow {
  index: number;
  oldValue: string;
  newValue: string;
  oldBytes: number;
  newBytes: number;
  diff: number;
}

export function SavePreviewDialog({
  modifications,
  preview,
  onConfirm,
  onCancel,
}: SavePreviewDialogProps) {
  const diffs = useMemo<DiffRow[]>(() => {
    if (!preview) return [];
    const rows: DiffRow[] = [];
    for (const [index, newValue] of modifications) {
      const entry = preview.strings.find((s) => s.index === index);
      const oldValue = entry?.value ?? "";
      const oldBytes = entry?.byte_length ?? 0;
      const newBytes = new TextEncoder().encode(newValue).length;
      rows.push({
        index,
        oldValue,
        newValue,
        oldBytes,
        newBytes,
        diff: newBytes - oldBytes,
      });
    }
    return rows.sort((a, b) => a.index - b.index);
  }, [modifications, preview]);

  return (
    <div className="dialog-overlay" onClick={onCancel}>
      <div className="dialog save-preview-dialog" onClick={(e) => e.stopPropagation()}>
        <div className="dialog-header">
          <span className="dialog-title">保存预览 / Save Preview</span>
          <button className="dialog-close" onClick={onCancel}>
            ×
          </button>
        </div>
        <div className="dialog-body">
          <div className="save-preview-summary">
            共 {diffs.length} 项修改 / {diffs.length} modification(s)
          </div>
          <div className="save-preview-table">
            <div className="diff-row diff-row-header">
              <span className="diff-cell-index">#</span>
              <span className="diff-cell-old">原值 / Old</span>
              <span className="diff-cell-arrow">→</span>
              <span className="diff-cell-new">新值 / New</span>
              <span className="diff-cell-bytes">字节 / Bytes</span>
            </div>
            {diffs.map((row) => (
              <div key={row.index} className="diff-row">
                <span className="diff-cell-index">#{row.index}</span>
                <span className="diff-cell-old" title={row.oldValue}>
                  {truncate(row.oldValue, 50)}
                </span>
                <span className="diff-cell-arrow">→</span>
                <span className="diff-cell-new" title={row.newValue}>
                  {truncate(row.newValue, 50)}
                </span>
                <span
                  className="diff-cell-bytes"
                  style={{
                    color: row.diff > 0 ? "var(--warning)" : "var(--text-muted)",
                  }}
                >
                  {row.oldBytes}→{row.newBytes}
                  {row.diff !== 0 && (
                    <span className="diff-delta">
                      ({row.diff > 0 ? "+" : ""}
                      {row.diff})
                    </span>
                  )}
                </span>
              </div>
            ))}
          </div>
        </div>
        <div className="save-preview-actions">
          <button className="btn" onClick={onCancel}>
            取消
          </button>
          <button className="btn btn-primary" onClick={onConfirm}>
            确认保存
          </button>
        </div>
      </div>
    </div>
  );
}

function truncate(s: string, max: number): string {
  if (s.length <= max) return s;
  return s.slice(0, max) + "…";
}
