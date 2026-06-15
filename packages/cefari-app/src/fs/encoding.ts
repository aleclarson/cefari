import type { FileData, FileEncoding, ReadFileOptions } from "./types.ts";

export function normalizeReadEncoding(
  options?: FileEncoding | ReadFileOptions,
): "utf8" | "base64" | null {
  if (typeof options === "string") return normalizeEncoding(options);
  if (!options?.encoding) return "base64";
  return normalizeEncoding(options.encoding);
}

export function normalizeEncoding(encoding: FileEncoding): "utf8" | "base64" {
  return encoding === "utf-8" ? "utf8" : encoding;
}

export function fileContents(
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

export function bytesToBase64(bytes: Uint8Array): string {
  let binary = "";
  for (let index = 0; index < bytes.length; index += 0x8000) {
    binary += String.fromCharCode(...bytes.subarray(index, index + 0x8000));
  }
  return btoa(binary);
}

export function base64ToBytes(encoded: string): Uint8Array {
  const binary = atob(encoded);
  const bytes = new Uint8Array(binary.length);
  for (let index = 0; index < binary.length; index += 1) {
    bytes[index] = binary.charCodeAt(index);
  }
  return bytes;
}

export function expectFileEmptyOrWritten(result: string): void {
  if (result !== "empty" && result !== "written") {
    throw new Error(`Unexpected file result: ${result}`);
  }
}
