import { useCallback, useState } from "react";
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
  const [error, setError] = useState<string | null>(null);
  const [saved, setSaved] = useState(true);

  const openPath = useCallback(async (path: string) => {
    setLoading(true);
    setError(null);
    try {
      const result = await invoke<FilePreview>("open_file", { path });
      if (result.kind === "class") {
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
    } catch (e: any) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  const openFile = useCallback(async () => {
    setLoading(true);
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
    setError(null);
    try {
      const preview = await invoke<ClassFilePreview>("open_class_in_jar", {
        jarPath,
        entryName,
      });
      setState((prev) => ({
        ...prev,
        source: { kind: "jar", jar_path: jarPath, entry_name: entryName },
        preview,
        modifications: new Map(),
      }));
      setSaved(true);
    } catch (e: any) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  const modify = useCallback((index: number, newValue: string) => {
    setState((prev) => {
      const mods = new Map(prev.modifications);
      mods.set(index, newValue);
      return { ...prev, modifications: mods };
    });
    setSaved(false);
  }, []);

  const revert = useCallback((index: number) => {
    setState((prev) => {
      const mods = new Map(prev.modifications);
      mods.delete(index);
      return { ...prev, modifications: mods };
    });
    setSaved(false);
  }, []);

  const save = useCallback(async () => {
    if (!state.source) return;
    setLoading(true);
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
      setSaved(true);
    } catch (e: any) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, [state.source, state.modifications]);

  return {
    state,
    loading,
    error,
    saved,
    openFile,
    openPath,
    openClassInJar,
    modify,
    revert,
    save,
  };
}
