import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { SearchHit, BatchReplacement } from "../types/classfile";

interface SearchPanelProps {
  /** 独立文件路径（class 模式） */
  filePath: string | null;
  /** JAR 路径（jar 模式） */
  jarPath: string | null;
  onClose: () => void;
  /** 保存成功后回调 */
  onSaved: () => void;
  /** 点击搜索结果跳转 */
  onNavigate?: (hit: SearchHit) => void;
}

export function SearchPanel({
  filePath,
  jarPath,
  onClose,
  onSaved,
  onNavigate,
}: SearchPanelProps) {
  const [query, setQuery] = useState("");
  const [replaceText, setReplaceText] = useState("");
  const [hits, setHits] = useState<SearchHit[]>([]);
  const [selected, setSelected] = useState<Set<number>>(new Set());
  const [searching, setSearching] = useState(false);
  const [replacing, setReplacing] = useState(false);
  const [status, setStatus] = useState<string>("");
  const [error, setError] = useState<string | null>(null);

  const isJar = !!jarPath;

  const handleSearch = useCallback(async () => {
    if (!query.trim()) return;
    setSearching(true);
    setError(null);
    setStatus("");
    try {
      const scope = jarPath
        ? { kind: "jar", jar_path: jarPath }
        : { kind: "file", path: filePath };
      const results = await invoke<SearchHit[]>("search_strings", {
        scope,
        query,
      });
      setHits(results);
      setSelected(new Set(results.map((_, i) => i)));
      setStatus(`找到 ${results.length} 个匹配 / ${results.length} match(es) found`);
    } catch (e: any) {
      setError(String(e));
    } finally {
      setSearching(false);
    }
  }, [query, jarPath, filePath]);

  const handleReplace = useCallback(async () => {
    if (selected.size === 0 || !replaceText) return;
    setReplacing(true);
    setError(null);
    try {
      // 收集选中的命中项，按条目分组
      const modsByEntry = new Map<string, BatchReplacement>();
      for (const i of selected) {
        const hit = hits[i];
        const key = hit.entry_name ?? "__file__";
        if (!modsByEntry.has(key)) {
          modsByEntry.set(key, {
            entry_name: hit.entry_name,
            modifications: [],
          });
        }
        modsByEntry.get(key)!.modifications.push({
          index: hit.index,
          new_value: hit.value.split(query).join(replaceText),
        });
      }

      const replacements = Array.from(modsByEntry.values());
      const path = jarPath ?? filePath!;
      const count = await invoke<number>("batch_save", {
        path,
        isJar,
        replacements,
      });

      setStatus(`已替换 ${count} 个条目中的 ${selected.size} 处字符串 / Replaced ${selected.size} occurrence(s) in ${count} entry(ies)`);
      setHits([]);
      setSelected(new Set());
      onSaved();
    } catch (e: any) {
      setError(String(e));
    } finally {
      setReplacing(false);
    }
  }, [selected, hits, replaceText, query, jarPath, filePath, isJar, onSaved]);

  const toggleSelect = (i: number) => {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(i)) next.delete(i);
      else next.add(i);
      return next;
    });
  };

  const toggleAll = () => {
    if (selected.size === hits.length) {
      setSelected(new Set());
    } else {
      setSelected(new Set(hits.map((_, i) => i)));
    }
  };

  return (
    <div className="dialog-overlay" onClick={onClose}>
      <div
        className="dialog search-panel-dialog"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="dialog-header">
          <span className="dialog-title">
            全局搜索替换 / Search & Replace
            {jarPath
              ? ` (JAR)`
              : filePath
                ? ` (Class)`
                : ""}
          </span>
          <button className="dialog-close" onClick={onClose}>
            ×
          </button>
        </div>
        <div className="dialog-body search-panel-body">
          {/* 搜索替换输入区 */}
          <div className="search-input-area">
            <div className="search-input-row">
              <input
                className="search-box"
                type="text"
                placeholder="搜索字符串…"
                value={query}
                onChange={(e) => setQuery(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") handleSearch();
                }}
              />
              <button
                className="btn btn-primary"
                onClick={handleSearch}
                disabled={searching || !query.trim()}
              >
                {searching ? "搜索中…" : "搜索"}
              </button>
            </div>
            <div className="search-input-row">
              <input
                className="search-box"
                type="text"
                placeholder="替换为…"
                value={replaceText}
                onChange={(e) => setReplaceText(e.target.value)}
              />
              <button
                className="btn btn-primary"
                onClick={handleReplace}
                disabled={
                  replacing ||
                  selected.size === 0 ||
                  !replaceText
                }
              >
                {replacing ? "替换中…" : `替换选中 (${selected.size})`}
              </button>
            </div>
          </div>

          {/* 状态信息 */}
          {status && (
            <div className="search-status">{status}</div>
          )}
          {error && (
            <div className="search-error">{error}</div>
          )}

          {/* 搜索结果 */}
          {hits.length > 0 && (
            <div className="search-results">
              <div className="search-results-header">
                <label className="search-check-all">
                  <input
                    type="checkbox"
                    checked={selected.size === hits.length}
                    onChange={toggleAll}
                  />
                  全选 / Select All
                </label>
                <span className="search-results-count">
                  {selected.size}/{hits.length}
                </span>
              </div>
              <div className="search-results-list">
                {hits.map((hit, i) => (
                  <div
                    key={`${hit.entry_name}-${hit.index}-${i}`}
                    className={`search-result-row ${selected.has(i) ? "selected" : ""}`}
                    onClick={() => toggleSelect(i)}
                  >
                    <input
                      type="checkbox"
                      checked={selected.has(i)}
                      onChange={() => toggleSelect(i)}
                    />
                    <div className="search-result-info">
                      <div className="search-result-meta">
                        <span className="search-result-class">
                          {hit.class_name || hit.source_label}
                        </span>
                        <span className="search-result-index">
                          #{hit.index}
                        </span>
                        <span className="search-result-bytes">
                          {hit.byte_length}B
                        </span>
                      </div>
                      <div className="search-result-preview" title={hit.value}>
                        {hit.match_preview}
                      </div>
                    </div>
                    {onNavigate && (
                      <button
                        className="search-result-jump"
                        title="跳转到此条目"
                        onClick={(e) => {
                          e.stopPropagation();
                          onNavigate(hit);
                        }}
                      >
                        →
                      </button>
                    )}
                  </div>
                ))}
              </div>
            </div>
          )}

          {hits.length === 0 && !searching && query.trim() && status && (
            <div className="empty-hint">无匹配结果 / No results</div>
          )}
        </div>
      </div>
    </div>
  );
}
