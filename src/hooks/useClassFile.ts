import { useCallback, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type {
  ClassFilePreview,
  ClassSource,
  FilePreview,
  Modification,
} from "../types/classfile";

export interface ClassState {
  source: ClassSource | null;
  preview: ClassFilePreview | null;
  /** 修改记录：常量池索引 -> 新值 */
  modifications: Map<number, string>;
  /** 文件名（显示用） */
  displayName: string;
  /** JAR 模式信息 */
  jarPath: string | null;
  jarSigned: boolean;
}

interface HistoryState {
  past: Map<number, string>[];
  future: Map<number, string>[];
}

export function useClassFile() {
  const [state, setState] = useState<ClassState>({
    source: null,
    preview: null,
    modifications: new Map(),
    displayName: "",
    jarPath: null,
    jarSigned: false,
  });
  const [loading, setLoading] = useState(false);
  const [loadingLabel, setLoadingLabel] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [saved, setSaved] = useState(true);
  const [history, setHistory] = useState<HistoryState>({ past: [], future: [] });
  /** 同步镜像 modifications，避免 setState 异步导致的历史记录不一致 */
  const modsRef = useRef<Map<number, string>>(new Map());

  const canUndo = history.past.length > 0;
  const canRedo = history.future.length > 0;

  /** 记录一次修改并推入历史栈 */
  const pushHistory = useCallback((prevMods: Map<number, string>) => {
    setHistory((h) => ({
      past: [...h.past, prevMods],
      future: [],
    }));
  }, []);

  /** 清空历史（打开文件/保存后调用） */
  const clearHistory = useCallback(() => {
    setHistory({ past: [], future: [] });
  }, []);

  const openPath = useCallback(async (path: string) => {
    setLoading(true);
    setLoadingLabel("正在打开文件…");
    setError(null);
    try {
      const result = await invoke<FilePreview>("open_file", { path });
      if (result.kind === "class") {
        modsRef.current = new Map();
        setState({
          source: { kind: "file", path },
          preview: result.preview,
          modifications: new Map(),
          displayName: path.split(/[\\/]/).pop() || path,
          jarPath: null,
          jarSigned: false,
        });
      } else {
        // jar: 不立即解析 class，仅展示文件树
        modsRef.current = new Map();
        setState({
          source: null,
          preview: null,
          modifications: new Map(),
          displayName: path.split(/[\\/]/).pop() || path,
          jarPath: result.info.path,
          jarSigned: result.info.is_signed,
        });
        // 保存 jar 文件树信息
        (window as any).__editclass_jar_tree = result.info;
      }
      setSaved(true);
      clearHistory();
    } catch (e: any) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, [clearHistory]);

  const openFile = useCallback(async () => {
    setLoading(true);
    setLoadingLabel("正在打开文件…");
    setError(null);
    try {
      const selected = await open({
        multiple: false,
        filters: [{ name: "Class / Jar", extensions: ["class", "jar"] }],
      });
      if (!selected) return;
      const path = Array.isArray(selected) ? selected[0] : selected;
      await openPath(path);
    } catch (e: any) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, [openPath]);

  const openClassInJar = useCallback(async (jarPath: string, entryName: string) => {
    setLoading(true);
    setLoadingLabel("正在加载类…");
    setError(null);
    try {
      const preview = await invoke<ClassFilePreview>("open_class_in_jar", {
        jarPath,
        entryName,
      });
      modsRef.current = new Map();
      setState((prev) => ({
        ...prev,
        source: { kind: "jar", jar_path: jarPath, entry_name: entryName },
        preview,
        modifications: new Map(),
      }));
      setSaved(true);
      clearHistory();
      return preview;
    } catch (e: any) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, [clearHistory]);

  const modify = useCallback((index: number, newValue: string) => {
    const prevMods = modsRef.current;
    const newMods = new Map(prevMods);
    newMods.set(index, newValue);
    modsRef.current = newMods;
    setState((prev) => ({ ...prev, modifications: newMods }));
    pushHistory(prevMods);
    setSaved(false);
  }, [pushHistory]);

  const revert = useCallback((index: number) => {
    const prevMods = modsRef.current;
    if (!prevMods.has(index)) return;
    const newMods = new Map(prevMods);
    newMods.delete(index);
    modsRef.current = newMods;
    setState((prev) => ({ ...prev, modifications: newMods }));
    pushHistory(prevMods);
    setSaved(false);
  }, [pushHistory]);

  const undo = useCallback(() => {
    setHistory((h) => {
      if (h.past.length === 0) return h;
      const previous = h.past[h.past.length - 1];
      const newPast = h.past.slice(0, -1);
      const currentMods = modsRef.current;
      modsRef.current = previous;
      setState((prev) => ({ ...prev, modifications: previous }));
      setSaved(false);
      return {
        past: newPast,
        future: [currentMods, ...h.future],
      };
    });
  }, []);

  const redo = useCallback(() => {
    setHistory((h) => {
      if (h.future.length === 0) return h;
      const next = h.future[0];
      const newFuture = h.future.slice(1);
      const currentMods = modsRef.current;
      modsRef.current = next;
      setState((prev) => ({ ...prev, modifications: next }));
      setSaved(false);
      return {
        past: [...h.past, currentMods],
        future: newFuture,
      };
    });
  }, []);

  const save = useCallback(async () => {
    if (!state.source) return;
    setLoading(true);
    setLoadingLabel("正在保存…");
    setError(null);
    try {
      const modifications: Modification[] = Array.from(
        state.modifications.entries()
      ).map(([index, new_value]) => ({ index, new_value }));
      if (state.source.kind === "file") {
        await invoke("save_class_file", {
          path: state.source.path,
          modifications,
        });
      } else {
        await invoke("save_class_in_jar", {
          jarPath: state.source.jar_path,
          entryName: state.source.entry_name,
          modifications,
        });
      }
      // 重新加载预览，使列表显示更新后的值
      const src = state.source;
      let preview: ClassFilePreview;
      if (src.kind === "file") {
        const r = await invoke<FilePreview>("open_file", { path: src.path });
        if (r.kind !== "class") throw new Error("重新加载失败");
        preview = r.preview;
      } else {
        preview = await invoke<ClassFilePreview>("open_class_in_jar", {
          jarPath: src.jar_path,
          entryName: src.entry_name,
        });
      }
      setState((prev) => ({
        ...prev,
        preview,
        modifications: new Map(),
      }));
      modsRef.current = new Map();
      clearHistory();
      setSaved(true);
    } catch (e: any) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, [state.source, state.modifications, clearHistory]);

  return {
    state,
    loading,
    loadingLabel,
    error,
    saved,
    canUndo,
    canRedo,
    openFile,
    openPath,
    openClassInJar,
    modify,
    revert,
    undo,
    redo,
    save,
    clearHistory,
  };
}
