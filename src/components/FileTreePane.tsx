import type { ClassState } from "../hooks/useClassFile";
import type { FileTreeNode } from "../types/classfile";
import { useState } from "react";

interface FileTreePaneProps {
  state: ClassState;
  onOpenClass: (jarPath: string, entryName: string) => void;
}

export function FileTreePane({ state, onOpenClass }: FileTreePaneProps) {
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
              node={jarInfo.file_tree}
              jarPath={state.jarPath}
              onOpenClass={onOpenClass}
              depth={0}
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
}

function TreeView({ node, jarPath, onOpenClass, depth }: TreeViewProps) {
  return (
    <div className="tree-children" style={{ paddingLeft: depth > 0 ? 12 : 0 }}>
      {node.children.map((child) => (
        <TreeItem
          key={child.path}
          node={child}
          jarPath={jarPath}
          onOpenClass={onOpenClass}
          depth={depth}
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
}: TreeViewProps) {
  const [expanded, setExpanded] = useState(depth < 1);
  const isClass = !node.is_dir && node.name.endsWith(".class");
  return (
    <div className="tree-node">
      <div
        className="tree-row"
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
        />
      )}
    </div>
  );
}
