import { useEffect, useState } from "react";
import type { StringEntry } from "../types/classfile";

interface EditPaneProps {
  entry: StringEntry | null;
  modifiedValue?: string;
  onModify: (index: number, newValue: string) => void;
  onRevert: (index: number) => void;
}

export function EditPane({
  entry,
  modifiedValue,
  onModify,
  onRevert,
}: EditPaneProps) {
  const [text, setText] = useState("");

  useEffect(() => {
    setText(modifiedValue !== undefined ? modifiedValue : entry?.value ?? "");
  }, [entry, modifiedValue]);

  if (!entry) {
    return (
      <div className="pane">
        <div className="pane-header">
          <span className="pane-title">编辑区</span>
        </div>
        <div className="pane-body">
          <div className="empty-hint">从左侧列表选择一个字符串条目</div>
        </div>
      </div>
    );
  }

  const isModified = modifiedValue !== undefined;
  const newBytes = new TextEncoder().encode(text).length;
  const diff = newBytes - entry.byte_length;
  const overflow = newBytes > 65535;
  // 仅允许修改字面量；非字面量（类名/方法名/描述符等）修改会破坏 class 结构
  const readonly = !entry.is_literal;

  return (
    <div className="pane">
      <div className="pane-header">
        <span className="pane-title">编辑区</span>
      </div>
      <div className="pane-body">
        <div className="edit-pane">
          <div className="edit-meta">
            <div className="edit-meta-item">
              <span className="edit-meta-label">索引</span>
              <span className="edit-meta-value">#{entry.index}</span>
            </div>
            <div className="edit-meta-item">
              <span className="edit-meta-label">类型</span>
              <span className="edit-meta-value">
                {entry.is_literal ? "字面量" : "Utf8"}
              </span>
            </div>
            <div className="edit-meta-item">
              <span className="edit-meta-label">原字节</span>
              <span className="edit-meta-value">{entry.byte_length} B</span>
            </div>
          </div>

          {readonly ? (
            <div className="risk-note">
              该条目为非字面量（类名/方法名/字段名/描述符），修改会破坏 class 结构，已锁定不可编辑。
            </div>
          ) : (
            <>
              <textarea
                className="edit-textarea"
                value={text}
                onChange={(e) => setText(e.target.value)}
                placeholder="输入新的字符串值…"
              />

              <div className="edit-bytes">
                <div className="byte-cell">
                  <div className="byte-cell-label">原字节</div>
                  <div className="byte-cell-value">{entry.byte_length} B</div>
                </div>
                <div className="byte-cell">
                  <div className="byte-cell-label">新字节</div>
                  <div className="byte-cell-value">{newBytes} B</div>
                </div>
                <div className="byte-cell">
                  <div className="byte-cell-label">差值</div>
                  <div
                    className="byte-cell-value"
                    style={{ color: diff > 0 ? "var(--warning)" : "var(--text)" }}
                  >
                    {diff > 0 ? `+${diff}` : diff} B
                  </div>
                </div>
              </div>
              {overflow && (
                <div className="byte-warn">
                  警告：新值字节长度超过 65535，class 文件无法容纳。
                </div>
              )}

              <div className="edit-actions">
                <button
                  className="btn btn-primary"
                  disabled={overflow || text === (modifiedValue ?? entry.value)}
                  onClick={() => onModify(entry.index, text)}
                >
                  应用
                </button>
                {isModified && (
                  <button
                    className="btn btn-ghost"
                    onClick={() => {
                      onRevert(entry.index);
                      setText(entry.value);
                    }}
                  >
                    还原
                  </button>
                )}
              </div>
            </>
          )}
        </div>
      </div>
    </div>
  );
}
