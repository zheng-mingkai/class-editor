/** 前端类型定义，与 Rust 后端结构对齐 */

export interface StringEntry {
  index: number;
  value: string;
  is_literal: boolean;
  byte_length: number;
}

export interface ClassFilePreview {
  class_name: string;
  version: string;
  strings: StringEntry[];
}

export interface JarEntry {
  name: string;
  size: number;
  compressed_size: number;
  is_dir: boolean;
}

export interface FileTreeNode {
  name: string;
  path: string;
  is_dir: boolean;
  children: FileTreeNode[];
  size: number | null;
}

export interface JarInfo {
  path: string;
  entries: JarEntry[];
  file_tree: FileTreeNode;
  is_signed: boolean;
  manifest: string | null;
}

export type FilePreview =
  | { kind: "class"; path: string; preview: ClassFilePreview }
  | { kind: "jar"; info: JarInfo };

export interface JdkInfo {
  path: string;
  version: string;
  source: "env" | "system" | "custom";
}

export interface DecompileResult {
  source: string;
  decompiler_version: string;
}

export interface TextRange {
  line: number;
  start_col: number;
  end_col: number;
}

export type ClassSource =
  | { kind: "file"; path: string }
  | { kind: "jar"; jar_path: string; entry_name: string };

export interface Modification {
  index: number;
  new_value: string;
}
