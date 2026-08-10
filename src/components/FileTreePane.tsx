import type { ClassState } from "../hooks/useClassFile";
import type { FileTreeNode } from "../types/classfile";
import { useState, useMemo } from "react";

interface FileTreePaneProps {
  state: ClassState;
  onOpenClass: (jarPath: string, entryName: string) => void;
  /** 当前选中的 JAR 内条目路径（用于高亮） */
  activeEntryName?: string | null;
}

/** 判断 node.path 是否是 targetPath 的祖先目录 */
function isAncestor(nodePath: string, targetPath: string | null | undefined): boolean {
  if (!targetPath) return false;
  return targetPath.startsWith(nodePath + "/");
}

export function FileTreePane({ state, onOpenClass, activeEntryName }: FileTreePaneProps) {
  const jarInfo = (window as any).__editclass_jar_tree;
  return (
    <div className="pane">
      <div className="pane-header">
        <span className="pane-title">文件树</span>
      </div>
      <div className="pane-body">
        {state.jarSigned && (
          <div className="warn-bar">
            此 JAR 已签名，修改后签名将失效。
          </div>
        )}
        {!state.displayName && (
          <div className="empty-hint">点击"打开文件"加载 .class 或 .jar</div>
        )}
        {state.displayName && !state.jarPath && (
          <div className="file-tree">
            <div className="tree-row active">
              <span className="tree-icon">●</span>
              {state.displayName}
            </div>
          </div>
        )}
        {state.jarPath && jarInfo?.file_tree && (
          <div className="file-tree">
            <div className="tree-row" style={{ fontWeight: 600 }}>
              <span className="tree-icon">📦</span>
              {state.displayName}
            </div>
            <TreeView
              key={activeEntryName ?? "none"}
              node={jarInfo.file_tree}
              jarPath={state.jarPath}
              onOpenClass={onOpenClass}
              depth={0}
              activeEntryName={activeEntryName}
            />
          </div>
        )}
      </div>
    </div>
  );
}

interface TreeViewProps {
  node: FileTreeNode;
  jarPath: string;
  onOpenClass: (jarPath: string, entryName: string) => void;
  depth: number;
  activeEntryName?: string | null;
}

function TreeView({ node, jarPath, onOpenClass, depth, activeEntryName }: TreeViewProps) {
  return (
    <div className="tree-children" style={{ paddingLeft: depth > 0 ? 12 : 0 }}>
      {node.children.map((child) => (
        <TreeItem
          key={child.path}
          node={child}
          jarPath={jarPath}
          onOpenClass={onOpenClass}
          depth={depth}
          activeEntryName={activeEntryName}
        />
      ))}
    </div>
  );
}

function TreeItem({
  node,
  jarPath,
  onOpenClass,
  depth,
  activeEntryName,
}: TreeViewProps) {
  // 初始展开：顶层默认展开，或者当前节点是 activeEntryName 的祖先
  // key 变化时整个 TreeView 重挂载，所以 useMemo([]) 能拿到正确的 activeEntryName
  const shouldExpandInitially = useMemo(
    () => depth < 1 || isAncestor(node.path, activeEntryName),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    []
  );
  const [expanded, setExpanded] = useState(shouldExpandInitially);
  const isClass = !node.is_dir && node.name.endsWith(".class");
  const isActive = activeEntryName === node.path;

  return (
    <div className="tree-node">
      <div
        className={`tree-row ${isActive ? "active" : ""}`}
        onClick={() => {
          if (node.is_dir) setExpanded((v) => !v);
          else if (isClass) onOpenClass(jarPath, node.path);
        }}
      >
        <span className="tree-icon">
          {node.is_dir ? (expanded ? "▾" : "▸") : isClass ? "☕" : "○"}
        </span>
        {node.name}
      </div>
      {node.is_dir && expanded && node.children.length > 0 && (
        <TreeView
          node={node}
          jarPath={jarPath}
          onOpenClass={onOpenClass}
          depth={depth + 1}
          activeEntryName={activeEntryName}
        />
      )}
    </div>
  );
}
