import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type { JdkInfo } from "../types/classfile";

interface SettingsDialogProps {
  onClose: () => void;
}

export function SettingsDialog({ onClose }: SettingsDialogProps) {
  const [jdkInfo, setJdkInfo] = useState<JdkInfo | null>(null);
  const [customPath, setCustomPath] = useState("");
  const [status, setStatus] = useState<string>("");
  const [loading, setLoading] = useState(false);

  // 加载时检测 JDK
  useEffect(() => {
    invoke<JdkInfo | null>("detect_jdk")
      .then((info) => {
        setJdkInfo(info);
        if (info) setStatus(`已检测到 JDK ${info.version}`);
        else setStatus("未检测到 JDK，请手动指定路径");
      })
      .catch((e) => setStatus(`检测失败：${e}`));
  }, []);

  const handlePickDir = async () => {
    const selected = await open({
      directory: true,
      multiple: false,
      title: "选择 JDK 根目录（包含 bin/java 的目录）",
    });
    if (selected && typeof selected === "string") {
      setCustomPath(selected);
    }
  };

  const handleApply = async () => {
    if (!customPath.trim()) {
      setStatus("请先填写或选择 JDK 路径");
      return;
    }
    setLoading(true);
    try {
      const info = await invoke<JdkInfo>("set_jdk_path", { path: customPath });
      setJdkInfo(info);
      setStatus(`✓ 已保存：JDK ${info.version}（${info.path}）`);
    } catch (e: any) {
      setStatus(`✗ ${e}`);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="dialog-overlay" onClick={onClose}>
      <div className="dialog" onClick={(e) => e.stopPropagation()}>
        <div className="dialog-header">
          <span className="dialog-title">设置</span>
          <button className="dialog-close" onClick={onClose}>×</button>
        </div>
        <div className="dialog-body">
          <div className="settings-section">
            <div className="settings-label">JDK 路径</div>
            <div className="settings-hint">
              反编译和字节码功能需要 JDK。系统已安装 JDK 且 JAVA_HOME 正确时可自动检测到；如未自动检测到，请手动指定 JDK 根目录（含 bin/java 的目录）。
            </div>

            {jdkInfo && (
              <div className="settings-current">
                <span className="settings-current-label">当前：</span>
                <span className="settings-current-value">
                  {jdkInfo.version} ({jdkInfo.path})
                </span>
                <span className="settings-source-tag">
                  {jdkInfo.source === "env"
                    ? "来自 JAVA_HOME"
                    : jdkInfo.source === "system"
                    ? "系统检测"
                    : "自定义"}
                </span>
              </div>
            )}

            <div className="settings-input-row">
              <input
                className="settings-input"
                placeholder="如：C:\Program Files\Java\jdk-17"
                value={customPath}
                onChange={(e) => setCustomPath(e.target.value)}
              />
              <button className="btn" onClick={handlePickDir}>
                浏览…
              </button>
              <button
                className="btn btn-primary"
                onClick={handleApply}
                disabled={loading || !customPath.trim()}
              >
                应用
              </button>
            </div>

            {status && <div className="settings-status">{status}</div>}
          </div>

          <div className="settings-section">
            <div className="settings-label">反编译器</div>
            <div className="settings-hint">
              内置 CFR 0.152 反编译器（已编译进 exe，无需额外配置）。
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
