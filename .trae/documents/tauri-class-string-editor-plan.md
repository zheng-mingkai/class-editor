# 编辑class 实施计划

## 概述

使用 **Tauri 2 + Rust + React + TypeScript** 构建跨平台桌面应用"编辑class"，用于替换 Java 编译后 `.class` 文件及 `.jar` 归档中的字符串。界面简洁直观，适配 Windows / macOS / Linux，支持跟随系统主题（浅色/深色自动切换）。

### 核心能力
1. **文件支持**：直接打开 `.class` 文件或 `.jar` 归档；JAR 模式下可浏览内部 class 列表并选择编辑。
2. **字符串编辑**：展示常量池中的字符串条目，支持编辑替换（字面量 + 可选全部 Utf8）。
3. **反编译源码**：通过内置 CFR 反编译器展示 Java 源码，选中字符串时联动高亮其在代码中的出现位置。
4. **字节码视图**：通过 JDK 自带 `javap -c` 展示字节码指令，供高级用户参考。

### 界面布局（整体左右布局，全分栏可拖动）
```
┌────────┬────────────────────────────────┐
│        │  反编译源码 / 字节码            │
│ 文件树 │  （代码区）                     │
│        │                                │
│  ←拖动 │═════════════ 拖动 ═════════════│ ← 上下拖动调整高度
│   ↕    ├──────────────┬─────────────────┤
│        │  字符串列表   │  编辑区域        │
│        │   拖动 ↔     │   拖动 ↔        │
└────────┴──────────────┴─────────────────┘
   ↑ 左右拖动调整宽度
```
**三条可拖动分隔线**（均支持拖动调整，位置持久化）：
1. **垂直分隔线 A**（文件树 ↔ 右侧区域）：左右拖动调整文件树宽度。
2. **水平分隔线 B**（代码区 ↔ 下方区域）：上下拖动调整代码区与下方区域的高度比例。
3. **垂直分隔线 C**（字符串列表 ↔ 编辑区域）：左右拖动调整两者宽度比例。

- 使用 `react-resizable-panels` 实现，面板嵌套结构：
  ```
  PanelGroup(horizontal)              // 分隔线 A
    ├─ Panel: 文件树
    └─ PanelGroup(vertical)           // 分隔线 B
         ├─ Panel: 代码区
         └─ PanelGroup(horizontal)    // 分隔线 C
              ├─ Panel: 字符串列表
              └─ Panel: 编辑区
  ```
- 各面板最小尺寸：文件树 ≥180px、编辑区 ≥220px、字符串列表 ≥200px、代码区高度 ≥120px、下方区域高度 ≥120px。
- 拖动位置通过 `localStorage` 持久化（`autoSaveId`），下次打开恢复。

### JAR 修改机制
- JAR 本质是 ZIP 归档，修改时采用**单条目替换**：只替换目标 class 条目，其余条目原样保留。
- **签名检测**：打开 JAR 时检查 `META-INF/*.SF|*.DSA|*.RSA`，若已签名则明确警告用户"修改后签名将失效"。
- **备份**：保存前自动创建 `.bak` 备份文件。
- **不保证 100% 成功**的场景：签名 JAR（签名失效）、sealed 包、class 结构被破坏。应用会通过警告与备份最大限度降低风险。

---

## 当前状态分析

- 工作目录 `d:\code\private\mingkai\editClassString` 为**空目录**，需从零搭建。
- 无现有代码、配置或依赖。
- 无历史记忆上下文。

### 关键技术调研结论

1. **Java class 文件格式**：常量池中的字符串以 `CONSTANT_Utf8_info` 结构存储，使用 **modified UTF-8** 编码（空字符 `\u0000` 编码为 `0xC0 0x80`），结构为 `u1 tag; u2 length; u1 bytes[length]`。`length` 为 u2（最大 65535 字节）。
2. **修改难点**：常量池是变长的，修改任一字符串长度后，后续所有偏移量都会变化，必须**整体重写 class 文件**。
3. **字符串分类**：
   - **字面量**：被 `CONSTANT_String_info`（tag=8）引用的 `Utf8` 条目，即代码中写死的 `String` 常量，修改最安全。
   - **其他 Utf8**：类名、方法名、字段名、描述符等，修改可能破坏类结构。
4. **Rust 生态**：存在 `classfile-parser` crate（仅解析，无序列化能力），但为保证修改可控性，采用**自研轻量解析/序列化模块**更稳妥。
5. **JDK 检测**：通过 `JAVA_HOME` 环境变量检测，辅以各操作系统常见路径回退，用于可选的 `javap` 预览/校验功能。

---

## 技术选型与决策

| 维度 | 选型 | 理由 |
|---|---|---|
| 桌面框架 | Tauri 2 | 体积小、性能高、Rust 后端原生支持 class 文件操作 |
| 前端框架 | React 18 + TypeScript | 用户指定，生态成熟，Tauri 官方模板支持 |
| 构建工具 | Vite 5 | Tauri 默认推荐，HMR 快 |
| UI 组件 | 原生 CSS + `react-resizable-panels` | 轻量自研组件 + 可拖动分栏面板库 |
| class 解析 | 自研 Rust 模块 | 需同时支持解析+序列化+修改，可控性最高 |
| JDK 检测 | Rust 标准库 + 环境变量 | 无额外依赖，跨平台 |
| 主题方案 | CSS 变量 + `prefers-color-scheme` | 跟随系统，零依赖 |

---

## 项目结构

```
editClassString/
├── src/                              # React 前端源码
│   ├── main.tsx                      # 应用入口
│   ├── App.tsx                       # 根组件（左右布局编排）
│   ├── components/
│   │   ├── TitleBar.tsx              # 窗口标题栏 + 主题切换
│   │   ├── Toolbar.tsx               # 工具栏（打开/保存/搜索/设置）
│   │   ├── FileTreePane.tsx          # 左侧全高：文件树 + JAR 目录结构 + 签名警告
│   │   ├── CodePane.tsx              # 右上：反编译源码 / 字节码（含语法高亮 + 联动）
│   │   ├── StringListPane.tsx        # 右下左：字符串列表（常驻显示）
│   │   ├── EditPane.tsx              # 右下右：编辑区域（字节信息 + 联动定位）
│   │   ├── StatusBar.tsx             # 底部状态栏
│   │   └── SettingsDialog.tsx        # 设置弹窗（JDK 路径配置）
│   ├── hooks/
│   │   ├── useTheme.ts               # 主题跟随系统逻辑
│   │   ├── useClassFile.ts           # class/jar 文件状态管理
│   │   └── useDecompile.ts           # 反编译结果与字符串位置映射
│   ├── styles/
│   │   ├── tokens.css                # 设计令牌（颜色/间距/圆角，含深色覆盖）
│   │   ├── code-theme.css            # 代码语法高亮配色（跟随主题）
│   │   └── global.css                # 全局样式
│   └── types/
│       └── classfile.ts              # 前端类型定义
├── src-tauri/                        # Rust 后端
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── build.rs
│   ├── resources/
│   │   └── cfr-0.152.jar             # 内置 CFR 反编译器（约 2MB）
│   ├── src/
│   │   ├── main.rs                   # Tauri 入口 + 命令注册
│   │   ├── classfile/
│   │   │   ├── mod.rs                # 模块入口
│   │   │   ├── parser.rs             # class 文件解析
│   │   │   ├── serializer.rs         # class 文件序列化（重写）
│   │   │   ├── constant_pool.rs      # 常量池结构与 Utf8 处理
│   │   │   └── mutf8.rs              # modified UTF-8 编解码
│   │   ├── jar.rs                    # JAR/ZIP 读取、单条目替换、签名检测
│   │   ├── commands.rs               # Tauri 命令（打开/保存/编辑/反编译）
│   │   ├── decompiler.rs             # CFR 调用 + 源码解析 + 字符串定位
│   │   └── jdk.rs                    # JDK 检测与路径管理
│   └── icons/
├── index.html
├── package.json
├── tsconfig.json
├── vite.config.ts
└── README.md                         # 不主动创建，按需
```

---

## 实施步骤

### 阶段 1：项目脚手架搭建

**目标**：初始化 Tauri 2 + React + TS 工程，确保可运行空窗口。

1. 使用 `npm create tauri-app@latest` 初始化项目（选择 React + TypeScript 模板）。
2. 配置 `tauri.conf.json`：
   - `productName`: "编辑class"
   - `identifier`: "com.editclass.app"
   - 窗口默认尺寸 1100×720，最小 900×600
   - 启用 `window.setDecorations` 跟随系统标题栏风格
3. 验证 `npm run tauri dev` 可启动空窗口。

**涉及文件**：`package.json`、`tauri.conf.json`、`vite.config.ts`、`src/main.tsx`、`src/App.tsx`、`index.html`、`src-tauri/Cargo.toml`、`src-tauri/src/main.rs`

---

### 阶段 2：Rust class 文件解析/序列化核心

**目标**：实现 class 文件的解析、修改、重写，这是应用的核心能力。

#### 2.1 modified UTF-8 编解码（`src-tauri/src/classfile/mutf8.rs`）
- 实现 `decode_mutf8(bytes: &[u8]) -> Result<String>`：将 modified UTF-8 字节解码为 Rust `String`。
- 实现 `encode_mutf8(s: &str) -> Result<Vec<u8>>`：将 Rust `String` 编码为 modified UTF-8 字节。
- 处理特殊规则：`\u0000` → `0xC0 0x80`，辅助平面字符用代理对编码。

#### 2.2 常量池结构（`src-tauri/src/classfile/constant_pool.rs`）
- 定义 `ConstantPoolEntry` 枚举，覆盖所有常量池类型（重点实现 `Utf8`、`String`、`Class`、`NameAndType`、`Methodref`、`Fieldref` 等）。
- 实现 `ConstantPool` 结构体：存储条目向量，提供按索引访问、查找被 `String_info` 引用的 Utf8（字面量分类）。

#### 2.3 解析器（`src-tauri/src/classfile/parser.rs`）
- 实现 `parse_classfile(bytes: &[u8]) -> Result<ClassFile>`：
  - 按 JVM 规范顺序解析：magic、version、constant_pool_count、constant_pool[]、access_flags、this_class、super_class、interfaces、fields、methods、attributes。
  - 保留各段的原始字节范围，便于未修改部分直接复用。
- 错误处理：非法 magic、截断文件、无效常量池索引等。

#### 2.4 序列化器（`src-tauri/src/classfile/serializer.rs`）
- 实现 `serialize_classfile(cf: &ClassFile) -> Result<Vec<u8>>`：
  - 按规范顺序重新输出字节流。
  - 当 Utf8 条目长度变化时，自动更新 `length` 字段并重排后续偏移（由于是顺序写入，偏移自然正确）。
- 关键点：所有 `u2`/`u4` 使用大端序写入。

**涉及文件**：`src-tauri/src/classfile/` 目录全部文件、`src-tauri/src/classfile/mod.rs`

---

### 阶段 3：JAR/ZIP 处理

**目标**：支持打开 JAR 文件、浏览内部 class 列表、单条目替换保存。

#### 3.1 JAR 读取（`src-tauri/src/jar.rs`）
- 添加 `zip` crate 依赖（`Cargo.toml`）。
- `list_jar_entries(path: &str) -> Result<Vec<JarEntry>>`：列出 JAR 中所有条目（路径 + 大小 + 压缩大小 + 是否目录）。
- `build_file_tree(entries: &[JarEntry]) -> FileTreeNode`：将扁平条目列表构建为目录树结构，供前端文件树组件渲染。
- `read_class_from_jar(jar_path: &str, entry_name: &str) -> Result<Vec<u8>>`：读取指定 class 条目的字节。
- `check_jar_signed(jar_path: &str) -> bool`：检查 `META-INF/` 下是否存在 `*.SF`、`*.DSA`、`*.RSA` 签名文件。

#### 3.2 JAR 单条目替换保存（`src-tauri/src/jar.rs`）
- `replace_class_in_jar(jar_path: &str, entry_name: &str, class_bytes: &[u8]) -> Result<()>`：
  - 使用 `zip` crate 打开原 JAR 读取所有条目。
  - 创建新 JAR（临时文件），将所有条目原样复制，仅替换目标 class 条目的内容。
  - 保留原 JAR 的压缩方式、注释等元信息。
  - 原子替换：写入临时文件成功后覆盖原文件。
  - 保存前自动创建 `xxx.jar.bak` 备份。

#### 3.3 数据结构
```rust
struct JarEntry {
    name: String,         // 如 "com/example/UserService.class"
    size: u64,            // 未压缩大小
    compressed_size: u64, // 压缩后大小
    is_dir: bool,         // 是否为目录
}
struct FileTreeNode {
    name: String,         // 节点名称
    path: String,         // 完整路径
    is_dir: bool,         // 是否目录
    children: Vec<FileTreeNode>,  // 子节点（仅目录有）
    size: Option<u64>,    // 文件大小（仅文件有）
}
struct JarInfo {
    path: String,
    entries: Vec<JarEntry>,
    file_tree: FileTreeNode,  // 构建好的目录树
    is_signed: bool,
    manifest: Option<String>,  // MANIFEST.MF 内容
}
```

**涉及文件**：`src-tauri/src/jar.rs`、`src-tauri/Cargo.toml`（添加 `zip` 依赖）

---

### 阶段 4：Tauri 命令层（前后端桥梁）

**目标**：暴露 Rust 能力给前端调用。

#### 4.1 JDK 检测（`src-tauri/src/jdk.rs`）
- `detect_jdk() -> Result<JdkInfo>`：
  - 优先读取 `JAVA_HOME` 环境变量。
  - 回退到操作系统常见路径：
    - Windows: `C:\Program Files\Java\*`
    - macOS: `/Library/Java/JavaVirtualMachines/*/Contents/Home`
    - Linux: `/usr/lib/jvm/*`
  - 返回 JDK 路径与版本（通过执行 `java -version` 解析）。
- 持久化用户自定义 JDK 路径（使用 `tauri-plugin-store` 或 JSON 配置文件存到 app data 目录）。

#### 4.2 命令定义（`src-tauri/src/commands.rs`）
- `open_file(path: String) -> Result<FilePreview>`：统一入口，根据扩展名分发：
  - `.class` → 解析 class 文件，返回元信息 + 字符串条目列表。
  - `.jar` → 列出 JAR 内 class 条目 + 签名状态，不立即解析 class。
- `open_class_in_jar(jar_path: String, entry_name: String) -> Result<ClassFilePreview>`：从 JAR 中读取指定 class 并解析。
- `save_class_file(path: String, modifications: Vec<Modification>) -> Result<()>`：保存修改到独立 .class 文件。保存前自动创建 `.bak` 备份。
- `save_class_in_jar(jar_path: String, entry_name: String, modifications: Vec<Modification>) -> Result<()>`：修改 class 后单条目替换回 JAR。保存前自动创建 `.jar.bak` 备份。
- `detect_jdk() -> Result<JdkInfo>`：返回检测到的 JDK 信息。
- `set_jdk_path(path: String) -> Result<JdkInfo>`：设置并验证自定义 JDK 路径。
- `decompile_class_file(path: String) -> Result<DecompileResult>`：调用内置 CFR 反编译，返回源码文本 + 字符串出现位置映射。
- `get_bytecode(path: String) -> Result<String>`：调用 `javap -c -p` 返回字节码文本。
- `verify_with_javap(path: String) -> Result<String>`（可选）：调用 JDK `javap -v` 输出完整反汇编，用于校验修改后的文件。

#### 4.3 命令注册（`src-tauri/src/main.rs`）
- 在 `tauri::generate_handler![]` 中注册上述命令。
- 配置文件对话框权限（`tauri.conf.json` 的 `allowlist` / capabilities）。

**涉及文件**：`src-tauri/src/commands.rs`、`src-tauri/src/jdk.rs`、`src-tauri/src/main.rs`、`src-tauri/Cargo.toml`（添加依赖）、`src-tauri/tauri.conf.json`

---

### 阶段 5：CFR 反编译器集成

**目标**：将 CFR 反编译器集成到应用中，支持反编译源码展示与字符串位置定位。

#### 5.1 CFR JAR 内置（`src-tauri/resources/cfr-0.152.jar`）
- 下载 CFR 0.152 release JAR（约 2MB）放入 `resources/` 目录。
- 在 `tauri.conf.json` 的 `bundle.resources` 中注册，确保打包时包含。
- 运行时通过 `tauri::path::resource_dir()` 获取 JAR 路径。

#### 5.2 反编译器调用（`src-tauri/src/decompiler.rs`）
- `decompile(class_bytes: &[u8], jdk_path: &str) -> Result<DecompileResult>`：
  - 将 class 字节写入临时文件（JAR 场景下从归档中提取后同样处理）。
  - 执行 `java -jar cfr.jar <temp_path> --comments false`（关闭注释噪音）。
  - 捕获 stdout 作为源码文本，完成后删除临时文件。
  - 处理失败情况（JDK 未找到、CFR JAR 缺失、反编译异常）。
- `locate_string_occurrences(source: &str, target: &str) -> Vec<TextRange>`：
  - 在反编译源码中搜索目标字符串的所有出现位置（行号 + 列范围）。
  - 用于前端联动高亮：选中字符串后高亮其在源码中的出现处。
- `get_bytecode(class_bytes: &[u8], jdk_path: &str) -> Result<String>`：
  - 执行 `javap -c -p <temp_path>`，返回字节码文本。

#### 5.3 数据结构
```rust
struct DecompileResult {
    source: String,              // 反编译源码全文
    occurrences: HashMap<u16, Vec<TextRange>>,  // 常量池索引 -> 源码中出现位置列表
    decompiler_version: String,  // CFR 版本号
}
struct TextRange {
    line: usize,      // 行号（1-based）
    start_col: usize, // 起始列
    end_col: usize,   // 结束列
}
```

**涉及文件**：`src-tauri/src/decompiler.rs`、`src-tauri/resources/cfr-0.152.jar`、`src-tauri/tauri.conf.json`、`src-tauri/Cargo.toml`

---

### 阶段 6：前端 UI 实现

**目标**：按视觉原型实现 2×2 上下分区界面，支持跟随系统主题。

#### 6.1 设计令牌（`src/styles/tokens.css`）
- 定义 CSS 变量：颜色、间距、圆角、字体（对齐视觉原型的令牌体系）。
- `:root` 浅色默认，`@media (prefers-color-scheme: dark)` 深色覆盖。
- 品牌色采用紫色系（`#4B3FE3` / `#6054F1`）。

#### 6.2 布局组件（`src/App.tsx`）
- 使用 `react-resizable-panels` 实现三条可拖动分隔线：
  - **分隔线 A**（垂直）：`PanelGroup direction="horizontal"` → 文件树面板 | 右侧面板组
  - **分隔线 B**（水平）：右侧 `PanelGroup direction="vertical"` → 代码区面板 | 下方面板组
  - **分隔线 C**（垂直）：下方 `PanelGroup direction="horizontal"` → 字符串列表面板 | 编辑区面板
- 各面板最小尺寸：文件树 ≥180px、编辑区 ≥220px、字符串列表 ≥200px、代码区高度 ≥120px、下方区域高度 ≥120px。
- 拖动位置通过 `localStorage` 持久化（`autoSaveId`），下次打开恢复。
- 顶部标题栏 + 工具栏，底部状态栏。
- 字符串列表常驻显示，不随代码视图切换而隐藏。

#### 6.3 各组件实现
- **TitleBar.tsx**：应用标题"编辑class" + 主题切换按钮（跟随系统/浅色/深色三态）。
- **Toolbar.tsx**：打开文件（.class/.jar）、保存按钮；搜索框；范围切换开关（仅字面量 / 全部 Utf8）；设置入口。
- **FileTreePane.tsx**（左侧全高）：文件树展示：
  - `.class` 模式：显示单个文件节点。
  - `.jar` 模式：按目录层级展示 JAR 内文件树，可展开/折叠文件夹，点击 `.class` 节点切换当前编辑的 class。
  - 顶部显示当前文件名；JAR 签名时显示警告条。
- **CodePane.tsx**（右上）：反编译源码 / 字节码切换标签；等宽字体代码渲染带行号；轻量语法高亮（关键字/字符串/注释/类型/数字），配色随主题切换；选中字符串时高亮其在源码中的所有出现位置（黄色标记）；点击高亮位置可跳转回字符串列表。
- **StringListPane.tsx**（右下左）：表格列：索引、类型标签（字面量/Utf8）、值、字节长度；选中行高亮；已修改行用品牌色标记；支持搜索过滤；常驻显示。
- **EditPane.tsx**（右下右）：显示选中条目的索引与类型；可编辑文本框；实时显示原字节/新字节/差值（超 65535 时警告）；应用/还原按钮；修改类名等敏感条目时给出风险提示；联动模式下显示"已定位到代码第 X 行"徽标。
- **StatusBar.tsx**：JDK 检测状态 + 路径；CFR 就绪状态；签名状态（JAR 模式）；已修改项数；保存状态。
- **SettingsDialog.tsx**：JDK 路径输入框 + 自动检测按钮 + 验证按钮；备份选项开关；反编译器信息展示（版本 + JAR 路径）。

#### 6.4 状态管理（`src/hooks/`）
- `useTheme.ts`：监听 `prefers-color-scheme` 变化，支持手动覆盖并持久化。
- `useClassFile.ts`：管理当前打开的 class/jar 文件状态、字符串列表、修改记录、保存状态；封装 Tauri `invoke` 调用；处理 JAR 模式下的 class 切换。
- `useDecompile.ts`：管理反编译结果、字节码文本、字符串出现位置映射；在打开/切换 class 后自动触发反编译；提供"根据常量池索引获取源码位置"的查询方法。

**涉及文件**：`src/` 下全部前端文件

---

### 阶段 7：跨平台适配与打包

**目标**：确保 Windows / macOS / Linux 均可运行与打包。

1. **窗口样式**：
   - Windows：使用系统原生标题栏（`decorations: true`）。
   - macOS：启用 `titleBarStyle: "Overlay"` 适配红绿灯按钮。
   - Linux：默认系统装饰。
2. **文件路径**：所有路径操作使用 `std::path::PathBuf`，避免硬编码分隔符。
3. **JDK 路径回退**：按平台分支处理（见阶段 3.1）。
4. **图标**：生成三平台所需图标格式（PNG/ICO/ICNS），使用 `tauri icon` 命令从单张源图生成。
5. **打包验证**：分别执行 `npm run tauri build` 在三平台生成安装包。

**涉及文件**：`src-tauri/tauri.conf.json`、`src-tauri/icons/`、`package.json`（scripts）

---

## 假设与决策

1. **class 文件修改策略**：采用"解析为结构体 → 修改 → 重新序列化"的完整重写方式，而非二进制 patch。原因：常量池变长，patch 不可行；完整重写可保证一致性。
2. **备份机制**：保存时自动在同目录生成 `xxx.class.bak`，可在设置中关闭。
3. **字面量识别**：通过遍历常量池中所有 `CONSTANT_String_info`（tag=8），收集其 `string_index` 指向的 Utf8 索引集合，即为字面量。
4. **范围切换默认值**：默认"仅字面量"，切换到"全部 Utf8"时对非字面量条目显示警告标识。
5. **JDK 用途**：用于反编译（`java -jar cfr.jar`）、字节码视图（`javap -c`）与可选校验（`javap -v`）；不参与 class 文件修改本身（Rust 独立完成）。
6. **反编译器选择**：采用 CFR 0.152（业界主流，单 JAR，输出质量高），打包内置到 `resources/` 目录，开箱即用，增加约 2MB 体积。
7. **字符串-代码联动**：反编译后由 Rust 端在源码中搜索各字符串字面量的出现位置，返回行号与列范围；前端据此高亮。点击高亮处可反向跳转到字符串列表。
8. **语法高亮**：自研轻量 Java 语法高亮（正则匹配关键字/字符串/注释/类型/数字），不引入 highlight.js / Prism 等库以保持轻量；配色随主题切换。
9. **UI 库使用**：仅引入 `react-resizable-panels`（可拖动分栏面板），其余组件自研 CSS，保持轻量与主题可控性。
10. **不引入状态管理库**：React Context + hooks 足够；避免过度工程化。
11. **最低 Rust 版本**：使用 Rust 1.70+（Tauri 2 要求）。
12. **反编译缓存**：打开文件时触发一次反编译，结果缓存在内存；修改字符串后不自动重新反编译（避免延迟），用户可手动点击"刷新反编译"按钮重新生成。

---

## 验证步骤

### 功能验证
1. **解析正确性**：准备一个已知内容的 `.class` 文件，对比应用解析出的字符串列表与 `javap -v` 输出的常量池 Utf8 条目，应完全一致。
2. **字面量分类**：确认代码中 `String s = "hello"` 的 `"hello"` 被标记为"字面量"，而类名/方法名被标记为"Utf8"。
3. **修改-保存-重读**：修改某字面量值 → 保存 → 重新打开该文件，确认修改已持久化且值正确。
4. **变长修改**：将短字符串改长（如 5 字节 → 50 字节），保存后用 `javap -v` 验证文件结构完整、无偏移错误。
5. **超长保护**：尝试输入超过 65535 字节的字符串，应被拒绝并提示。
6. **modified UTF-8**：测试包含中文、emoji、空字符的字符串，确认编解码正确。
7. **备份验证**：保存后确认 `.bak` 文件存在且内容为修改前版本。
8. **反编译展示**：打开 class 文件后切换到"反编译源码"标签，确认 CFR 正常输出 Java 源码，语法高亮正确。
9. **字符串联动高亮**：在字符串列表选中某字面量 → 切换到反编译视图，确认源码中该字符串的出现位置被高亮；编辑面板显示"已定位到第 X 行"。
10. **反向跳转**：在反编译视图中点击高亮的字符串 → 应回到字符串列表并选中对应条目。
11. **字节码视图**：切换到"字节码"标签，确认 `javap -c -p` 输出正确显示。
12. **反编译刷新**：修改字符串后点击"刷新反编译"，确认源码中对应字符串已更新。

### 跨平台验证
13. **JDK 检测**：在 Windows/macOS/Linux 分别验证 `JAVA_HOME` 存在与不存在时的检测逻辑。
14. **主题跟随**：切换操作系统主题，确认应用界面与代码高亮自动切换浅色/深色。
15. **打包**：三平台分别执行 `npm run tauri build`，确认生成可运行的安装包且 CFR JAR 已正确打包。
16. **分栏拖动**：拖动文件树/代码区/字符串列表/编辑区的分隔线，确认可自由调整宽度且最小宽度限制生效；重启后恢复上次拖动位置。
17. **JAR 文件树**：打开 JAR 后确认目录树正确展示层级结构，可展开/折叠，点击 class 节点可切换编辑。

### 安全性验证
18. **非法文件**：打开非 class/jar 文件时应给出明确错误提示，不崩溃。
19. **损坏 class**：打开被截断的 class 文件时应报错并指引原因。
20. **损坏 JAR**：打开损坏的 JAR 文件时应报错并指引原因。
21. **敏感修改警告**：修改类名/方法名/描述符时，UI 应明确提示风险。
22. **CFR 缺失**：模拟 CFR JAR 丢失场景，应用应给出明确提示而非崩溃。
23. **JDK 缺失**：未检测到 JDK 时，反编译与字节码功能应禁用并提示用户配置 JDK，字符串编辑功能不受影响。
24. **签名 JAR 警告**：打开已签名 JAR 时应显示明显警告，保存时再次确认。
