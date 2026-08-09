import { Panel, PanelGroup, PanelResizeHandle } from "react-resizable-panels";
import { useEffect, useState } from "react";
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
import type { StringEntry } from "./types/classfile";

export default function App() {
  const { mode, setMode } = useTheme();
  const classFile = useClassFile();
  const decompile = useDecompile(classFile.state.source);
  const [selected, setSelected] = useState<StringEntry | null>(null);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [dragOver, setDragOver] = useState(false);

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
    if (entry) decompile.locate(entry.value);
  };

  return (
    <div className="app-shell">
      <TitleBar mode={mode} onThemeChange={setMode} />
      <Toolbar
        onOpen={classFile.openFile}
        onSave={classFile.save}
        saved={classFile.saved}
        loading={classFile.loading}
        hasFile={!!classFile.state.source}
        onOpenSettings={() => setSettingsOpen(true)}
      />
      <div className="app-body" style={{ position: "relative" }}>
        <PanelGroup direction="horizontal" autoSaveId="editclass-root-h">
          {/* 分隔线 A 左侧：文件树 */}
          <Panel defaultSize={20} minSize={14} order={1}>
            <FileTreePane
              state={classFile.state}
              onOpenClass={classFile.openClassInJar}
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
    </div>
  );
}
