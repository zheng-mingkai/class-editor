import { useMemo, useRef, useEffect } from "react";
import type { TextRange } from "../types/classfile";

interface CodePaneProps {
  state: {
    source: string;
    bytecode: string;
    version: string;
    loading: boolean;
    error: string | null;
  };
  view: "source" | "bytecode";
  onViewChange: (v: "source" | "bytecode") => void;
  className?: string;
  occurrences: TextRange[];
}

export function CodePane({
  state,
  view,
  onViewChange,
  className,
  occurrences,
}: CodePaneProps) {
  const content = view === "source" ? state.source : state.bytecode;
  const highlightLines = useMemo(
    () => new Set(occurrences.map((o) => o.line)),
    [occurrences]
  );
  const lines = content ? content.split("\n") : [];
  const codeViewRef = useRef<HTMLDivElement>(null);
  const lineRefs = useRef<(HTMLDivElement | null)[]>([]);

  // 当 occurrences 变化时，滚动到第一个高亮行
  useEffect(() => {
    if (occurrences.length === 0) return;
    const firstLine = occurrences[0].line;
    const el = lineRefs.current[firstLine - 1];
    if (el && codeViewRef.current) {
      el.scrollIntoView({ behavior: "smooth", block: "center" });
    }
  }, [occurrences]);

  return (
    <div className="pane">
      <div className="pane-header">
        <div className="code-tabs">
          <button
            className={`code-tab ${view === "source" ? "active" : ""}`}
            onClick={() => onViewChange("source")}
          >
            反编译源码
          </button>
          <button
            className={`code-tab ${view === "bytecode" ? "active" : ""}`}
            onClick={() => onViewChange("bytecode")}
          >
            字节码
          </button>
        </div>
        {className && <span className="code-class-label">{className}</span>}
        {state.version && (
          <span className="code-class-label">{state.version}</span>
        )}
      </div>
      <div className="pane-body">
        {state.error && (
          <div className="empty-hint" style={{ color: "var(--danger)" }}>
            {state.error}
          </div>
        )}
        {!state.loading && !content && !state.error && (
          <div className="empty-hint">打开 class 文件后显示反编译源码</div>
        )}
        {content && (
          <div className="code-view" ref={codeViewRef}>
            {lines.map((line, i) => (
              <div
                key={i}
                ref={(el) => { lineRefs.current[i] = el; }}
                className={`code-line ${highlightLines.has(i + 1) ? "highlight" : ""}`}
              >
                <span className="code-line-num">{i + 1}</span>
                <span dangerouslySetInnerHTML={{ __html: tokenize(line) }} />
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

/** 轻量 Java 语法高亮。 */
function tokenize(line: string): string {
  // 先转义 HTML
  const esc = line
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
  // 注释
  if (/^\s*\/\//.test(esc)) return `<span class="tok-com">${esc}</span>`;
  // 字符串
  let out = esc.replace(
    /("(?:[^"\\]|\\.)*")/g,
    '<span class="tok-str">$1</span>'
  );
  // 关键字
  out = out.replace(
    /\b(public|private|protected|static|final|void|class|interface|extends|implements|return|new|if|else|for|while|try|catch|throw|throws|import|package|this|super|null|true|false|int|long|double|float|boolean|char|byte|short|enum|abstract|synchronized|volatile|transient|native|instanceof)\b/g,
    '<span class="tok-kw">$1</span>'
  );
  // 数字
  out = out.replace(/\b(\d+\.?\d*[fFdDlL]?)\b/g, '<span class="tok-num">$1</span>');
  return out;
}
