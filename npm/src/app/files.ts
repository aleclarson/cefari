import { fs } from "./fs.ts";
import { invokeFile } from "./results.ts";

export type AppDataDir = {
  rootKind: string;
  displayPath: string;
};

export type JsonValue =
  | null
  | boolean
  | number
  | string
  | JsonValue[]
  | { [key: string]: JsonValue };

export type WriteJsonOptions = {
  createParents?: boolean;
  overwrite?: boolean;
  space?: number | string;
};

export type ObjectUrlOptions = {
  type?: string;
};

export type FilesApi = {
  appDataDir(): Promise<AppDataDir>;
  readText(path: string): Promise<string>;
  writeText(path: string, contents: string): Promise<void>;
  readBytes(path: string): Promise<Uint8Array>;
  writeBytes(path: string, contents: Uint8Array | ArrayBuffer): Promise<void>;
  exists(path: string): Promise<boolean>;
  readJson<T extends JsonValue = JsonValue>(path: string): Promise<T>;
  writeJson(path: string, value: JsonValue, options?: WriteJsonOptions): Promise<void>;
  toObjectUrl(path: string, options?: ObjectUrlOptions): Promise<string>;
};

async function appDataDir(): Promise<AppDataDir> {
  const result = await invokeFile({
    command: "files",
    payload: { file: "appDataDir" },
  });
  if (result.result !== "appDataDir") {
    throw new Error(`Unexpected file result: ${result.result}`);
  }
  return result.payload;
}

async function readText(path: string): Promise<string> {
  return await fs.readFile(path, "utf8");
}

async function writeText(path: string, contents: string): Promise<void> {
  await fs.writeFile(path, contents);
}

async function readBytes(path: string): Promise<Uint8Array> {
  return await fs.readFile(path);
}

async function writeBytes(
  path: string,
  contents: Uint8Array | ArrayBuffer,
): Promise<void> {
  await fs.writeFile(path, contents);
}

async function exists(path: string): Promise<boolean> {
  const result = await invokeFile({
    command: "files",
    payload: { file: "exists", payload: { path } },
  });
  if (result.result !== "exists") {
    throw new Error(`Unexpected file result: ${result.result}`);
  }
  return result.payload.exists;
}

async function readJson<T extends JsonValue = JsonValue>(
  path: string,
): Promise<T> {
  return JSON.parse(await readText(path)) as T;
}

async function writeJson(
  path: string,
  value: JsonValue,
  options: WriteJsonOptions = {},
): Promise<void> {
  await fs.writeFile(path, `${JSON.stringify(value, null, options.space)}\n`, {
    createParents: options.createParents,
    overwrite: options.overwrite,
  });
}

async function toObjectUrl(
  path: string,
  options: ObjectUrlOptions = {},
): Promise<string> {
  const bytes = await readBytes(path);
  const buffer = new ArrayBuffer(bytes.byteLength);
  new Uint8Array(buffer).set(bytes);
  return URL.createObjectURL(new Blob([buffer], { type: options.type }));
}

export const files: Readonly<FilesApi> = Object.freeze({
  appDataDir,
  readText,
  writeText,
  readBytes,
  writeBytes,
  exists,
  readJson,
  writeJson,
  toObjectUrl,
});
