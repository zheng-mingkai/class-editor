<div align="center">

# 编辑class / EditClass

**一个用于替换 Java `.class` / `.jar` 文件中字符串字面量的跨平台桌面应用**
**A cross-platform desktop app to replace string literals inside compiled Java `.class` / `.jar` files**

基于 Tauri 2 + Rust + React 18 构建 · Windows · macOS · Linux
Built with Tauri 2 + Rust + React 18

[![Release](https://img.shields.io/badge/release-v0.1.0-blue)](https://github.com/zheng-mingkai/editClassString/releases/tag/v0.1.0)
[![License](https://img.shields.io/badge/license-MIT-green)](./LICENSE)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey)]()

</div>

---

## 📖 简介 / Introduction

### 中文

`编辑class` 是一个桌面工具，可以直接修改 Java 编译后 `.class` 文件（或 `.jar` 归档中某个条目）的常量池字符串字面量。无需重新编译源码，无需反编译再编译——直接在字节码层面替换字符串，配合内置 CFR 反编译器实时查看代码效果。

**适用场景**：快速修复硬编码 URL、文案、配置 key，国际化调整，调试期临时改值，CTF / 逆向辅助等。

### English

`EditClass` is a desktop tool that directly edits string literals in the constant pool of compiled Java `.class` files (or individual entries inside a `.jar` archive). No source recompilation, no decompile-then-recompile round-trip — strings are replaced at the bytecode level, with a built-in CFR decompiler showing the code context in real time.

**Use cases**: quick fixes to hardcoded URLs / copy / config keys, i18n tweaks, debug-time value patching, CTF / reverse-engineering assistance.

---

## ✨ 核心功能 / Features

| | 中文 | English |
|---|------|---------|
| ✏️ | **字面量编辑**：仅允许修改 `CONSTANT_String_info` 引用的字面量，类名/方法名/描述符等结构化 Utf8 条目自动锁定，避免破坏 class 结构。 | **Literal editing**: only `String`-info-referenced Utf8 entries are editable; structural Utf8 (class/method/field names, descriptors) is locked to prevent breaking the class. |
| 📂 | **JAR 支持**：浏览 JAR 内目录树，单条目替换保存，其余条目原样保留。 | **JAR support**: browse the JAR internal tree, replace a single entry in place, all other entries preserved. |
| 🔍 | **CFR 反编译 + javap 字节码**：内置 CFR 0.152，一键切换源码 / 字节码视图；选中字面量后联动高亮代码中的所有出现位置（支持 Java 转义匹配）。 | **CFR decompile + javap bytecode**: CFR 0.152 built in, toggle source / bytecode view; selecting a literal highlights all occurrences in code (Java-escape aware). |
| 🧱 | **可拖动分栏布局**：文件树 / 代码区 / 字面量列表 / 编辑区四区，三条分隔线均可拖动调整，位置 `localStorage` 持久化。 | **Resizable panel layout**: four panes (file tree / code / literal list / editor) with three draggable dividers, positions persisted to `localStorage`. |
| 📥 | **拖拽打开**：把 `.class` / `.jar` 文件拖到窗口任意位置即可打开。 | **Drag to open**: drop a `.class` / `.jar` anywhere on the window to open it. |
| ⚙️ | **JDK 自动检测**：自动从 `JAVA_HOME` 或系统常见路径检测 JDK，也可在设置中手动指定并持久化。 | **JDK auto-detect**: auto-detected from `JAVA_HOME` or platform-specific default paths; manually configurable in Settings and persisted. |
| 🛡️ | **安全机制**：保存前自动创建 `.bak` 备份；打开已签名 JAR 时明确警告"修改后签名失效"。 | **Safety**: automatic `.bak` backup before saving; signed JARs trigger an explicit "signature will be invalidated" warning. |
| 🎨 | **跟随系统主题**：浅色 / 深色自动切换，基于 CSS 变量方案，零额外依赖。 | **System theme**: light / dark auto-switching via CSS variables, zero extra deps. |

---

## 🖼️ 界面布局 / Layout

```
┌────────┬────────────────────────────────┐
│        │  反编译源码 / 字节码            │
│ 文件树 │  Decompiled source / bytecode  │
│        │                                │
│  ←拖动 │═════════════ 拖动 ═════════════│  ← 上下拖动 / vertical drag
│   ↕    ├──────────────┬─────────────────┤
│        │  字面量列表   │  编辑区域        │
│        │ Literal list  │  Editor          │
│        │   拖动 ↔      │   拖动 ↔         │
└────────┴──────────────┴─────────────────┘
   ↑ 左右拖动 / horizontal drag
```

---

## 📥 下载安装 / Download & Install

### Windows

从 Release 页面下载 `class-v0.1.0-windows-x64.exe`，双击即可运行，无需安装。

Download `class-v0.1.0-windows-x64.exe` from the [Releases page](https://github.com/zheng-mingkai/editClassString/releases), double-click to run — no installer required.

**运行需求 / Requirements**:
- Windows 10 / 11 (x64)
- WebView2 Runtime（Windows 10/11 通常已预装 / usually preinstalled）
- 反编译 / 字节码功能需要 JDK（编辑字符串本身不需要 / JDK only needed for decompile / bytecode view）

### macOS / Linux

v0.1.0 仅提供 Windows 预编译包。macOS / Linux 用户请从源码构建：

v0.1.0 ships a Windows build only. macOS / Linux users please build from source:

```bash
git clone https://github.com/zheng-mingkai/editClassString.git
cd editClassString
npm install
npm run tauri build
```

---

## 🚀 使用说明 / Usage

### 中文

1. **打开文件**：点击工具栏"打开文件"按钮，或直接把 `.class` / `.jar` 拖入窗口。
2. **浏览 JAR（可选）**：JAR 模式下，左侧文件树展开目录，点击 `.class` 节点切换当前编辑的类。
3. **查看代码**：右上区域切换"反编译源码" / "字节码"标签，查看当前类的 Java 源码或 `javap -c` 输出。
4. **选中字面量**：右下左的列表中点击任意字面量条目，编辑区加载其内容，代码区高亮所有出现位置。
5. **编辑并应用**：在编辑区修改文本，查看原/新字节长度对比，点击"应用"暂存修改。
6. **保存**：点击工具栏"保存"按钮，原文件同目录生成 `.bak` 备份，修改写入原文件。

### English

1. **Open a file**: click "Open" in the toolbar, or drag a `.class` / `.jar` directly into the window.
2. **Browse JAR (optional)**: in JAR mode, expand the file tree on the left and click a `.class` node to switch the active class.
3. **View code**: in the top-right area, toggle the "Source" / "Bytecode" tab to see CFR-decompiled Java or `javap -c` output.
4. **Select a literal**: click any literal entry in the bottom-left list — the editor loads it, and the code area highlights every occurrence.
5. **Edit & apply**: edit the text in the editor, compare original vs. new byte length, click "Apply" to stage the change.
6. **Save**: click "Save" in the toolbar — a `.bak` backup is created next to the original, and the modification is written in place.

---

## 🛠️ 技术栈 / Tech Stack

| 层 / Layer | 技术 / Technology |
|---|---|
| 桌面框架 / Desktop | Tauri 2 |
| 后端 / Backend | Rust（自研 class 解析/序列化 + modified UTF-8 编解码 + ZIP 单条目替换） |
| 前端 / Frontend | React 18 + TypeScript + Vite 5 |
| 分栏布局 / Layout | react-resizable-panels |
| 反编译器 / Decompiler | CFR 0.152（二进制嵌入 exe，首次运行释放到临时目录） |
| 主题 / Theme | CSS 变量 + `prefers-color-scheme` |

---

## 📁 项目结构 / Project Structure

```
editClassString/
├── src/                          # React 前端 / React frontend
│   ├── components/               #   UI 组件 / UI components
│   ├── hooks/                    #   状态管理 / state hooks
│   ├── styles/                   #   样式 / styles (tokens/theme/global)
│   └── types/                    #   类型定义 / TS types
├── src-tauri/                    # Rust 后端 / Rust backend
│   ├── src/
│   │   ├── classfile/            #   class 解析/序列化 / parse & serialize
│   │   │   ├── mutf8.rs          #     modified UTF-8 编解码
│   │   │   ├── constant_pool.rs  #     常量池结构 / constant pool
│   │   │   ├── parser.rs         #     解析器 / parser
│   │   │   └── serializer.rs     #     序列化器 / serializer
│   │   ├── jar.rs                #   JAR 读取/单条目替换/签名检测
│   │   ├── decompiler.rs         #   CFR + javap 调用
│   │   ├── jdk.rs                #   JDK 检测 / JDK detection
│   │   └── commands.rs           #   Tauri 命令 / Tauri commands
│   └── resources/
│       └── cfr-0.152.jar         #   内置 CFR / bundled CFR
├── package.json
└── vite.config.ts
```

---

## 💻 开发 / Development

### 环境要求 / Prerequisites

- [Node.js](https://nodejs.org/) ≥ 18
- [Rust](https://www.rust-lang.org/tools/install) (stable)
- 系统依赖 / System deps:
  - **Windows**: WebView2（Win10/11 已预装）+ MSVC Build Tools
  - **macOS**: Xcode Command Line Tools
  - **Linux**: `webkit2gtk-4.1`、`libssl-dev`、`librsvg2-dev` 等

### 本地运行 / Run locally

```bash
npm install
npm run tauri dev
```

### 构建打包 / Build

```bash
npm run tauri build
```

产物路径 / Output:
- Windows: `src-tauri/target/release/edit-class.exe`
- macOS: `src-tauri/target/release/bundle/dmg/*.dmg`
- Linux: `src-tauri/target/release/bundle/deb/*.deb` 或 `/appimage/*.AppImage`

---

## ⚠️ 注意事项 / Caveats

- **仅修改字面量**：非字面量 Utf8 条目（类名、方法名、字段名、描述符）不可编辑，以避免破坏 class 结构。
  **Literals only**: non-literal Utf8 entries (class / method / field names, descriptors) are not editable, to avoid breaking class structure.
- **签名 JAR**：修改已签名 JAR 后签名将失效，应用会警告并自动备份，但不保证 100% 可用（sealed 包、结构损坏等极端情况）。
  **Signed JARs**: modifying a signed JAR invalidates its signature; the app warns and backs up, but cannot guarantee success in edge cases (sealed packages, corrupted structures).
- **字节长度限制**：单个 Utf8 条目最长 65535 字节（JVM 规范），超出时编辑区会显示警告。
  **Byte length limit**: a single Utf8 entry is capped at 65535 bytes per the JVM spec; the editor warns on overflow.

---

## 📜 License

MIT License — 见 [LICENSE](./LICENSE) 文件。
See the [LICENSE](./LICENSE) file for details.

---

<div align="center">

**[⬇ 下载 / Download](https://github.com/zheng-mingkai/editClassString/releases/latest)** · **[🐛 报告问题 / Report issue](https://github.com/zheng-mingkai/editClassString/issues)**

</div>
