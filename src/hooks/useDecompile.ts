import { useEffect, useState, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { ClassSource, DecompileResult, TextRange } from "../types/classfile";

interface DecompileState {
  source: string;
  bytecode: string;
  version: string;
  loading: boolean;
  loadingLabel: string;
  error: string | null;
  /** 选中字符串在源码中的出现位置 */
  occurrences: TextRange[];
}

export function useDecompile(source: ClassSource | null) {
  const [state, setState] = useState<DecompileState>({
    source: "",
    bytecode: "",
    version: "",
    loading: false,
    loadingLabel: "",
    error: null,
    occurrences: [],
  });
  const [view, setView] = useState<"source" | "bytecode">("source");
  /** 常量池索引 → 源码行号数组映射（由后端 javap -c -l 解析） */
  const indexLineMap = useRef<Map<number, number[]>>(new Map());

  useEffect(() => {
    if (!source) {
      setState({
        source: "",
        bytecode: "",
        version: "",
        loading: false,
        loadingLabel: "",
        error: null,
        occurrences: [],
      });
      indexLineMap.current.clear();
      return;
    }
    let cancelled = false;
    setState((s) => ({ ...s, loading: true, loadingLabel: "正在反编译…", error: null }));
    invoke<DecompileResult>("decompile_class", { source })
      .then((res) => {
        if (cancelled) return;
        setState((s) => ({
          ...s,
          source: res.source,
          version: res.decompiler_version,
          loading: false,
        }));
      })
      .catch((e) => {
        if (cancelled) return;
        setState((s) => ({ ...s, loading: false, error: String(e) }));
      });

    // 异步加载常量池索引 → 行号数组映射（不阻塞反编译）
    invoke<Record<string, number[]>>("locate_string_lines", { source })
      .then((map) => {
        if (cancelled) return;
        const m = new Map<number, number[]>();
        for (const [k, v] of Object.entries(map)) {
          m.set(Number(k), v);
        }
        indexLineMap.current = m;
      })
      .catch(() => {
        // 行号映射加载失败，静默回退到文本匹配
      });

    return () => {
      cancelled = true;
    };
  }, [source]);

  /** 保存后重新加载反编译源码和行号映射 */
  const reload = useCallback(() => {
    if (!source) return;
    setState((s) => ({ ...s, loading: true, loadingLabel: "正在反编译…", bytecode: "", occurrences: [] }));
    invoke<DecompileResult>("decompile_class", { source })
      .then((res) => {
        setState((s) => ({
          ...s,
          source: res.source,
          version: res.decompiler_version,
          loading: false,
        }));
      })
      .catch((e) => {
        setState((s) => ({ ...s, loading: false, error: String(e) }));
      });
    invoke<Record<string, number[]>>("locate_string_lines", { source })
      .then((map) => {
        const m = new Map<number, number[]>();
        for (const [k, v] of Object.entries(map)) {
          m.set(Number(k), v);
        }
        indexLineMap.current = m;
      })
      .catch(() => {});
  }, [source]);

  const loadBytecode = async () => {
    if (!source || state.bytecode) return;
    setState((s) => ({ ...s, loading: true, loadingLabel: "正在加载字节码…", error: null }));
    try {
      const code = await invoke<string>("get_bytecode", { source });
      setState((s) => ({ ...s, bytecode: code, loading: false }));
    } catch (e: any) {
      setState((s) => ({ ...s, loading: false, error: String(e) }));
    }
  };

  const switchView = (v: "source" | "bytecode") => {
    setView(v);
    if (v === "bytecode" && !state.bytecode) loadBytecode();
  };

  /** 根据常量池索引和字符串值定位源码位置。
   *  优先用后端 javap 解析的精确行号数组，在这些行中查找列位置；
   *  回退到全文文本匹配。 */
  const locate = (index: number, value: string) => {
    const exactLines = indexLineMap.current.get(index);
    if (exactLines && exactLines.length > 0) {
      const ranges = computeOccurrencesInLines(state.source, value, exactLines);
      setState((s) => ({ ...s, occurrences: ranges }));
      return;
    }
    // 回退到文本匹配
    const ranges = computeOccurrences(state.source, value);
    setState((s) => ({ ...s, occurrences: ranges }));
  };

  return { state, view, switchView, locate, reload };
}

/** 在指定行中搜索目标字符串的出现位置（仅在双引号内的字符串字面量中查找）。 */
function computeOccurrencesInLines(source: string, target: string, lineNums: number[]): TextRange[] {
  const ranges: TextRange[] = [];
  if (!target || lineNums.length === 0) return ranges;
  const candidates = new Set([target, javaEscape(target)]);
  const lines = source.split("\n");
  const lineSet = new Set(lineNums);
  const seen = new Set<string>();
  for (let i = 0; i < lines.length; i++) {
    if (!lineSet.has(i + 1)) continue;
    const line = lines[i];
    for (const seg of extractStringLiterals(line)) {
      for (const t of candidates) {
        let start = 0;
        while (true) {
          const idx = seg.text.indexOf(t, start);
          if (idx < 0) break;
          const col = seg.startCol + idx;
          const key = `${i + 1}:${col}`;
          if (!seen.has(key)) {
            seen.add(key);
            ranges.push({ line: i + 1, start_col: col, end_col: col + t.length });
          }
          start = idx + t.length;
        }
      }
    }
  }
  return ranges;
}

function computeOccurrences(source: string, target: string): TextRange[] {
  const ranges: TextRange[] = [];
  if (!target) return ranges;
  // 字符串列表显示的是 unescape 后的真实值（含真实换行/双引号），
  // 而反编译源码里的字符串字面量是 Java 转义形式（\n、\"、\\）。
  // 同时尝试两种形式查找并去重，确保高亮能命中。
  const candidates = new Set([target, javaEscape(target)]);
  const lines = source.split("\n");
  const seen = new Set<string>();
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    // 只在双引号内的字符串字面量中搜索，避免匹配变量名/类名/import 等
    for (const seg of extractStringLiterals(line)) {
      for (const t of candidates) {
        let start = 0;
        while (true) {
          const idx = seg.text.indexOf(t, start);
          if (idx < 0) break;
          const col = seg.startCol + idx;
          const key = `${i + 1}:${col}`;
          if (!seen.has(key)) {
            seen.add(key);
            ranges.push({ line: i + 1, start_col: col, end_col: col + t.length });
          }
          start = idx + t.length;
        }
      }
    }
  }
  return ranges;
}

/** 从一行代码中提取双引号内的字符串字面量段（返回相对列号和内容）。 */
function extractStringLiterals(line: string): { text: string; startCol: number }[] {
  const result: { text: string; startCol: number }[] = [];
  let i = 0;
  while (i < line.length) {
    // 跳到下一个双引号（非转义）
    const dq = line.indexOf('"', i);
    if (dq < 0) break;
    // 收集引号内的内容（处理 \" 转义）
    let j = dq + 1;
    let text = "";
    while (j < line.length) {
      if (line[j] === "\\" && j + 1 < line.length) {
        text += line[j] + line[j + 1];
        j += 2;
      } else if (line[j] === '"') {
        break;
      } else {
        text += line[j];
        j++;
      }
    }
    result.push({ text, startCol: dq + 1 });
    i = j + 1;
  }
  return result;
}

/** 将字符串转为 Java 字面量中的转义形式（与 CFR 反编译输出一致）。 */
function javaEscape(s: string): string {
  let out = "";
  for (const ch of s) {
    const cp = ch.codePointAt(0)!;
    if (cp > 0x7e || cp < 0x20) {
      if (ch === "\n") out += "\\n";
      else if (ch === "\r") out += "\\r";
      else if (ch === "\t") out += "\\t";
      else {
        // 非 ASCII 字符以 \uXXXX 形式输出（CFR 默认行为）
        for (const unit of [...ch]) {
          const u = unit.codePointAt(0)!;
          out += `\\u${u.toString(16).padStart(4, "0")}`;
        }
      }
    } else if (ch === "\\") {
      out += "\\\\";
    } else if (ch === '"') {
      out += '\\"';
    } else {
      out += ch;
    }
  }
  return out;
}
