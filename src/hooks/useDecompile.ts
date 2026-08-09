import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { ClassSource, DecompileResult, TextRange } from "../types/classfile";

interface DecompileState {
  source: string;
  bytecode: string;
  version: string;
  loading: boolean;
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
    error: null,
    occurrences: [],
  });
  const [view, setView] = useState<"source" | "bytecode">("source");

  useEffect(() => {
    if (!source) {
      setState({
        source: "",
        bytecode: "",
        version: "",
        loading: false,
        error: null,
        occurrences: [],
      });
      return;
    }
    let cancelled = false;
    setState((s) => ({ ...s, loading: true, error: null }));
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
    // 字节码按需加载，不在此触发
    return () => {
      cancelled = true;
    };
  }, [source]);

  const loadBytecode = async () => {
    if (!source || state.bytecode) return;
    setState((s) => ({ ...s, loading: true, error: null }));
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

  /** 根据字符串值定位源码位置（前端本地计算）。 */
  const locate = (value: string) => {
    const ranges = computeOccurrences(state.source, value);
    setState((s) => ({ ...s, occurrences: ranges }));
  };

  return { state, view, switchView, locate };
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
    for (const t of candidates) {
      let start = 0;
      while (true) {
        const idx = line.indexOf(t, start);
        if (idx < 0) break;
        const key = `${i + 1}:${idx}`;
        if (!seen.has(key)) {
          seen.add(key);
          ranges.push({ line: i + 1, start_col: idx, end_col: idx + t.length });
        }
        start = idx + t.length;
      }
    }
  }
  return ranges;
}

/** 将字符串转为 Java 字面量中的转义形式（与 CFR 反编译输出一致）。 */
function javaEscape(s: string): string {
  return s
    .replace(/\\/g, "\\\\")
    .replace(/"/g, '\\"')
    .replace(/\n/g, "\\n")
    .replace(/\r/g, "\\r")
    .replace(/\t/g, "\\t");
}
