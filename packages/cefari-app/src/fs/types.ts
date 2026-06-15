import type { DirEntry, FileKind, FileStat } from "../ipc.ts";

export type FileEncoding = "utf8" | "utf-8" | "base64";
export type FileData = string | Uint8Array | ArrayBuffer;

export type ReadFileOptions = {
  encoding?: FileEncoding | null;
};

export type WriteFileOptions = {
  createParents?: boolean;
  encoding?: FileEncoding;
  overwrite?: boolean;
};

export type ReaddirOptions = {
  withFileTypes?: boolean;
};

export type MkdirOptions = {
  recursive?: boolean;
};

export type RmOptions = {
  force?: boolean;
  recursive?: boolean;
};

export class CefariDirent {
  readonly name: string;
  readonly path: string;
  readonly kind: FileKind;

  constructor(entry: DirEntry) {
    this.name = entry.name;
    this.path = entry.path;
    this.kind = entry.kind;
  }

  isFile(): boolean {
    return this.kind === "file";
  }

  isDirectory(): boolean {
    return this.kind === "directory";
  }

  isSymbolicLink(): boolean {
    return this.kind === "symlink";
  }
}

export class CefariStats {
  readonly path: string;
  readonly kind: FileKind;
  readonly size: number;
  readonly mtimeMs: number | null;
  readonly birthtimeMs: number | null;

  constructor(stat: FileStat) {
    this.path = stat.path;
    this.kind = stat.kind;
    this.size = stat.size ?? 0;
    this.mtimeMs = stat.modifiedAtMs;
    this.birthtimeMs = stat.createdAtMs;
  }

  isFile(): boolean {
    return this.kind === "file";
  }

  isDirectory(): boolean {
    return this.kind === "directory";
  }

  isSymbolicLink(): boolean {
    return this.kind === "symlink";
  }
}

export type FileSystemApi = {
  readFile(path: string): Promise<Uint8Array>;
  readFile(path: string, encoding: FileEncoding): Promise<string>;
  readFile(
    path: string,
    options: ReadFileOptions,
  ): Promise<string | Uint8Array>;
  writeFile(
    path: string,
    data: FileData,
    options?: WriteFileOptions,
  ): Promise<void>;
  readdir(path?: string): Promise<string[]>;
  readdir(
    path: string,
    options: { withFileTypes: true },
  ): Promise<CefariDirent[]>;
  readdir(
    path: string,
    options: ReaddirOptions,
  ): Promise<string[] | CefariDirent[]>;
  mkdir(path: string, options?: MkdirOptions): Promise<void>;
  rm(path: string, options?: RmOptions): Promise<void>;
  rename(from: string, to: string): Promise<void>;
  copyFile(from: string, to: string): Promise<void>;
  stat(path: string): Promise<CefariStats>;
  access(path: string): Promise<boolean>;
};
