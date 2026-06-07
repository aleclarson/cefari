import type { DirEntry, FileKind, FileStat } from "./ipc.ts";
import { invokeFile } from "./results.ts";

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
  readFile(path: string, options: ReadFileOptions): Promise<string | Uint8Array>;
  writeFile(path: string, data: FileData, options?: WriteFileOptions): Promise<void>;
  readdir(path?: string): Promise<string[]>;
  readdir(path: string, options: { withFileTypes: true }): Promise<CefariDirent[]>;
  readdir(path: string, options: ReaddirOptions): Promise<string[] | CefariDirent[]>;
  mkdir(path: string, options?: MkdirOptions): Promise<void>;
  rm(path: string, options?: RmOptions): Promise<void>;
  rename(from: string, to: string): Promise<void>;
  copyFile(from: string, to: string): Promise<void>;
  stat(path: string): Promise<CefariStats>;
  access(path: string): Promise<boolean>;
};

function readFile(path: string): Promise<Uint8Array>;
function readFile(path: string, encoding: FileEncoding): Promise<string>;
function readFile(
  path: string,
  options: ReadFileOptions,
): Promise<string | Uint8Array>;
async function readFile(
  path: string,
  options?: FileEncoding | ReadFileOptions,
): Promise<string | Uint8Array> {
  const encoding = normalizeReadEncoding(options);
  const result = await invokeFile({
    command: "files",
    payload: {
      file: "readFile",
      payload: { path, encoding },
    },
  });

  if (result.result === "text") return result.payload.contents;
  if (result.result === "base64") return base64ToBytes(result.payload.contents);
  throw new Error(`Unexpected file result: ${result.result}`);
}

async function writeFile(
  path: string,
  data: FileData,
  options: WriteFileOptions = {},
): Promise<void> {
  const result = await invokeFile({
    command: "files",
    payload: {
      file: "writeFile",
      payload: {
        path,
        contents: fileContents(data, options.encoding),
        options: {
          createParents: options.createParents ?? true,
          overwrite: options.overwrite ?? true,
        },
      },
    },
  });
  expectFileEmptyOrWritten(result.result);
}

function readdir(path?: string): Promise<string[]>;
function readdir(
  path: string,
  options: { withFileTypes: true },
): Promise<CefariDirent[]>;
function readdir(
  path: string,
  options: ReaddirOptions,
): Promise<string[] | CefariDirent[]>;
async function readdir(
  path = ".",
  options: ReaddirOptions = {},
): Promise<string[] | CefariDirent[]> {
  const result = await invokeFile({
    command: "files",
    payload: {
      file: "readdir",
      payload: { path, withFileTypes: options.withFileTypes ?? false },
    },
  });

  if (result.result !== "dirEntries") {
    throw new Error(`Unexpected file result: ${result.result}`);
  }

  if (options.withFileTypes) {
    return result.payload.entries.map((entry) => new CefariDirent(entry));
  }
  return result.payload.entries.map((entry) => entry.name);
}

async function mkdir(path: string, options: MkdirOptions = {}): Promise<void> {
  const result = await invokeFile({
    command: "files",
    payload: {
      file: "mkdir",
      payload: { path, recursive: options.recursive ?? false },
    },
  });
  expectFileEmptyOrWritten(result.result);
}

async function rm(path: string, options: RmOptions = {}): Promise<void> {
  const result = await invokeFile({
    command: "files",
    payload: {
      file: "rm",
      payload: {
        path,
        force: options.force ?? false,
        recursive: options.recursive ?? false,
      },
    },
  });
  expectFileEmptyOrWritten(result.result);
}

async function rename(from: string, to: string): Promise<void> {
  const result = await invokeFile({
    command: "files",
    payload: { file: "rename", payload: { from, to } },
  });
  expectFileEmptyOrWritten(result.result);
}

async function copyFile(from: string, to: string): Promise<void> {
  const result = await invokeFile({
    command: "files",
    payload: { file: "copyFile", payload: { from, to } },
  });
  expectFileEmptyOrWritten(result.result);
}

async function stat(path: string): Promise<CefariStats> {
  const result = await invokeFile({
    command: "files",
    payload: { file: "stat", payload: { path } },
  });
  if (result.result !== "stat") {
    throw new Error(`Unexpected file result: ${result.result}`);
  }
  return new CefariStats(result.payload);
}

async function access(path: string): Promise<boolean> {
  const result = await invokeFile({
    command: "files",
    payload: { file: "access", payload: { path } },
  });
  if (result.result !== "access") {
    throw new Error(`Unexpected file result: ${result.result}`);
  }
  return result.payload.ok;
}

function normalizeReadEncoding(
  options?: FileEncoding | ReadFileOptions,
): "utf8" | "base64" | null {
  if (typeof options === "string") return normalizeEncoding(options);
  if (!options?.encoding) return "base64";
  return normalizeEncoding(options.encoding);
}

function normalizeEncoding(encoding: FileEncoding): "utf8" | "base64" {
  return encoding === "utf-8" ? "utf8" : encoding;
}

function fileContents(
  data: FileData,
  encoding: FileEncoding = "utf8",
): { kind: "text"; value: string } | { kind: "base64"; value: string } {
  if (typeof data === "string") {
    return normalizeEncoding(encoding) === "base64"
      ? { kind: "base64", value: data }
      : { kind: "text", value: data };
  }
  return { kind: "base64", value: bytesToBase64(toBytes(data)) };
}

function toBytes(data: Uint8Array | ArrayBuffer): Uint8Array {
  return data instanceof Uint8Array ? data : new Uint8Array(data);
}

function bytesToBase64(bytes: Uint8Array): string {
  let binary = "";
  for (let index = 0; index < bytes.length; index += 0x8000) {
    binary += String.fromCharCode(...bytes.subarray(index, index + 0x8000));
  }
  return btoa(binary);
}

function base64ToBytes(encoded: string): Uint8Array {
  const binary = atob(encoded);
  const bytes = new Uint8Array(binary.length);
  for (let index = 0; index < binary.length; index += 1) {
    bytes[index] = binary.charCodeAt(index);
  }
  return bytes;
}

function expectFileEmptyOrWritten(result: string): void {
  if (result !== "empty" && result !== "written") {
    throw new Error(`Unexpected file result: ${result}`);
  }
}

export const fs: Readonly<FileSystemApi> = Object.freeze({
  readFile,
  writeFile,
  readdir,
  mkdir,
  rm,
  rename,
  copyFile,
  stat,
  access,
});
