import { useState } from "react";
import type { ClassFilePreview, StringEntry } from "../types/classfile";

interface StringListPaneProps {
  preview: ClassFilePreview | null;
  modifications: Map<number, string>;
  selected: StringEntry | null;
  onSelect: (entry: StringEntry | null) => void;
}

export function StringListPane({
  preview,
  modifications,
  selected,
  onSelect,
}: StringListPaneProps) {
  const [filter, setFilter] = useState("");

  // 仅展示字面量（可编辑条目）；非字面量不允许修改，故不在列表中展示
  const allLiteral = (preview?.strings || []).filter((s) => s.is_literal);
  const rows = allLiteral.filter((s) => {
    if (filter && !s.value.toLowerCase().includes(filter.toLowerCase()))
      return false;
    return true;
  });

  return (
    <div className="pane string-list">
      <div className="pane-header">
        <span className="pane-title">
          字面量列表 {preview && `(${rows.length})`}
        </span>
      </div>
      <input
        className="search-box"
        placeholder="搜索字符串…"
        value={filter}
        onChange={(e) => setFilter(e.target.value)}
      />
      <div className="string-table">
        {!preview && <div className="empty-hint">暂无数据</div>}
        {rows.map((s) => {
          const modified = modifications.has(s.index);
          const displayValue = modified ? modifications.get(s.index)! : s.value;
          return (
            <div
              key={s.index}
              className={`string-row ${selected?.index === s.index ? "active" : ""} ${modified ? "modified" : ""}`}
              onClick={() => onSelect(s)}
            >
              <span className="string-index">#{s.index}</span>
              <span className="tag tag-literal">字面量</span>
              <span className="string-value" title={displayValue}>
                {displayValue}
              </span>
              <span className="string-bytes">{s.byte_length}B</span>
            </div>
          );
        })}
      </div>
    </div>
  );
}
