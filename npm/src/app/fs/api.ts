import { invokeFile } from "../results.ts";
import {
  base64ToBytes,
  expectFileEmptyOrWritten,
  fileContents,
  normalizeReadEncoding,
} from "./encoding.ts";
import {
  CefariDirent,
  CefariStats,
  type FileData,
  type FileEncoding,
  type FileSystemApi,
  type MkdirOptions,
  type ReaddirOptions,
  type ReadFileOptions,
  type RmOptions,
  type WriteFileOptions,
} from "./types.ts";

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
