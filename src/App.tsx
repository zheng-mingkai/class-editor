import { Panel, PanelGroup, PanelResizeHandle } from "react-resizable-panels";
import { useEffect, useState, useCallback } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useTheme } from "./hooks/useTheme";
import { useClassFile } from "./hooks/useClassFile";
import { useDecompile } from "./hooks/useDecompile";
import { TitleBar } from "./components/TitleBar";
import { Toolbar } from "./components/Toolbar";
import { FileTreePane } from "./components/FileTreePane";
import { CodePane } from "./components/CodePane";
import { StringListPane } from "./components/StringListPane";
import { EditPane } from "./components/EditPane";
import { StatusBar } from "./components/StatusBar";
import { SettingsDialog } from "./components/SettingsDialog";
import { SavePreviewDialog } from "./components/SavePreviewDialog";
import { SearchPanel } from "./components/SearchPanel";
import type { StringEntry, SearchHit } from "./types/classfile";

export default function App() {
  const { mode, setMode } = useTheme();
  const classFile = useClassFile();
  const decompile = useDecompile(classFile.state.source);
  const [selected, setSelected] = useState<StringEntry | null>(null);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [dragOver, setDragOver] = useState(false);
  const [savePreviewOpen, setSavePreviewOpen] = useState(false);
  const [searchOpen, setSearchOpen] = useState(false);

  // 保存前先弹出 Diff 预览
  const handleSave = useCallback(() => {
    if (classFile.state.modifications.size > 0) {
      setSavePreviewOpen(true);
    } else {
      classFile.save();
    }
  }, [classFile]);

  // 全局快捷键
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      const mod = e.ctrlKey || e.metaKey;
      if (!mod) return;
      const key = e.key.toLowerCase();
      if (key === "z" && !e.shiftKey) {
        e.preventDefault();
        classFile.undo();
      } else if (key === "y" || (key === "z" && e.shiftKey)) {
        e.preventDefault();
        classFile.redo();
      } else if (key === "s") {
        e.preventDefault();
        handleSave();
      } else if (key === "f") {
        e.preventDefault();
        setSearchOpen(true);
      } else if (key === "o") {
        e.preventDefault();
        classFile.openFile();
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [classFile, handleSave]);

  // 确认保存
  const handleConfirmSave = useCallback(async () => {
    setSavePreviewOpen(false);
    await classFile.save();
    // 保存后重新反编译 + 更新选中条目
    decompile.reload();
    if (selected) {
      const updated = classFile.state.preview?.strings.find((s) => s.index === selected.index);
      if (updated) {
        setSelected(updated);
        decompile.locate(updated.index, updated.value);
      }
    }
  }, [classFile, decompile, selected]);

  // 全局搜索替换保存后重新加载
  const handleSearchSaved = useCallback(() => {
    if (classFile.state.source) {
      // 重新加载当前打开的文件
      if (classFile.state.source.kind === "file") {
        classFile.openPath(classFile.state.source.path);
      } else if (classFile.state.jarPath) {
        classFile.openPath(classFile.state.jarPath);
      }
    }
  }, [classFile]);

  // 点击搜索结果跳转：打开对应 class + 选中字符串条目
  const handleSearchNavigate = useCallback(async (hit: SearchHit) => {
    let preview = classFile.state.preview;
    // JAR 模式：如果当前打开的不是该条目，先切换
    if (hit.jar_path && hit.entry_name) {
      const currentEntry = classFile.state.source?.kind === "jar"
        ? classFile.state.source.entry_name
        : null;
      if (currentEntry !== hit.entry_name) {
        preview = await classFile.openClassInJar(hit.jar_path, hit.entry_name) ?? preview;
      }
    }
    // 选中对应的字符串条目
    if (preview) {
      const entry = preview.strings.find((s) => s.index === hit.index);
      if (entry) {
        setSelected(entry);
        decompile.locate(entry.index, entry.value);
      }
    }
  }, [classFile, decompile]);

  // 监听 Tauri 原生文件拖放事件（不依赖 web dragover，更可靠）
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    const win = getCurrentWindow();
    win.onDragDropEvent((event) => {
      if (event.payload.type === "drop") {
        const paths = event.payload.paths;
        if (paths && paths.length > 0) {
          const p = paths[0];
          if (p.toLowerCase().endsWith(".class") || p.toLowerCase().endsWith(".jar")) {
            classFile.openPath(p);
          }
        }
        setDragOver(false);
      } else if (event.payload.type === "enter" || event.payload.type === "over") {
        setDragOver(true);
      } else if (event.payload.type === "leave") {
        setDragOver(false);
      }
    }).then((fn) => {
      unlisten = fn;
    });
    return () => {
      if (unlisten) unlisten();
    };
  }, [classFile]);

  const handleSelect = (entry: StringEntry | null) => {
    setSelected(entry);
    if (entry) decompile.locate(entry.index, entry.value);
  };

  // 全局阻塞遮罩：打开文件 / 反编译 / 保存等阻塞操作期间显示
  const isLoading = classFile.loading || decompile.state.loading;
  const loadingLabel = classFile.loading
    ? classFile.loadingLabel
    : decompile.state.loadingLabel;

  return (
    <div className="app-shell">
      {isLoading && (
        <div className="global-loading-overlay">
          <div className="spinner" />
          <div className="loading-text">{loadingLabel || "处理中…"}</div>
        </div>
      )}
      <TitleBar mode={mode} onThemeChange={setMode} />
      <Toolbar
        onOpen={classFile.openFile}
        onSave={handleSave}
        saved={classFile.saved}
        loading={classFile.loading}
        hasFile={!!classFile.state.source}
        canUndo={classFile.canUndo}
        canRedo={classFile.canRedo}
        onUndo={classFile.undo}
        onRedo={classFile.redo}
        onOpenSettings={() => setSettingsOpen(true)}
        onOpenSearch={() => setSearchOpen(true)}
      />
      <div className="app-body" style={{ position: "relative" }}>
        <PanelGroup direction="horizontal" autoSaveId="editclass-root-h">
          {/* 分隔线 A 左侧：文件树 */}
          <Panel defaultSize={20} minSize={14} order={1}>
            <FileTreePane
              state={classFile.state}
              onOpenClass={classFile.openClassInJar}
              activeEntryName={
                classFile.state.source?.kind === "jar"
                  ? classFile.state.source.entry_name
                  : null
              }
            />
          </Panel>
          <PanelResizeHandle id="divider-a" />
          {/* 分隔线 A 右侧：上下分区 */}
          <Panel defaultSize={80} minSize={50} order={2}>
            <PanelGroup direction="vertical" autoSaveId="editclass-right-v">
              {/* 分隔线 B 上方：代码区 */}
              <Panel defaultSize={55} minSize={18} order={1}>
                <CodePane
                  state={decompile.state}
                  view={decompile.view}
                  onViewChange={decompile.switchView}
                  className={classFile.state.preview?.class_name}
                  occurrences={decompile.state.occurrences}
                />
              </Panel>
              <PanelResizeHandle id="divider-b" />
              {/* 分隔线 B 下方：字符串列表 + 编辑区 */}
              <Panel defaultSize={45} minSize={18} order={2}>
                <PanelGroup direction="horizontal" autoSaveId="editclass-bottom-h">
                  <Panel defaultSize={45} minSize={20} order={1}>
                    <StringListPane
                      preview={classFile.state.preview}
                      modifications={classFile.state.modifications}
                      selected={selected}
                      onSelect={handleSelect}
                    />
                  </Panel>
                  <PanelResizeHandle id="divider-c" />
                  <Panel defaultSize={55} minSize={25} order={2}>
                    <EditPane
                      entry={selected}
                      modifiedValue={
                        selected
                          ? classFile.state.modifications.get(selected.index)
                          : undefined
                      }
                      onModify={classFile.modify}
                      onRevert={classFile.revert}
                    />
                  </Panel>
                </PanelGroup>
              </Panel>
            </PanelGroup>
          </Panel>
        </PanelGroup>
        {dragOver && <div className="drag-overlay">拖放 .class / .jar 文件以打开</div>}
      </div>
      <StatusBar
        saved={classFile.saved}
        modifiedCount={classFile.state.modifications.size}
        jarSigned={classFile.state.jarSigned}
        error={classFile.error}
      />
      {settingsOpen && (
        <SettingsDialog onClose={() => setSettingsOpen(false)} />
      )}
      {savePreviewOpen && (
        <SavePreviewDialog
          modifications={classFile.state.modifications}
          preview={classFile.state.preview}
          onConfirm={handleConfirmSave}
          onCancel={() => setSavePreviewOpen(false)}
        />
      )}
      {searchOpen && (
        <SearchPanel
          filePath={
            classFile.state.source?.kind === "file"
              ? classFile.state.source.path
              : null
          }
          jarPath={classFile.state.jarPath}
          onClose={() => setSearchOpen(false)}
          onSaved={handleSearchSaved}
          onNavigate={handleSearchNavigate}
        />
      )}
    </div>
  );
}
