import type {
  CefariIpcCommand,
  CefariIpcError,
  CefariIpcResult,
} from "./ipc.ts";

export type CefariErrorCode = CefariIpcError["code"];

export class CefariError extends Error {
  override readonly name = "CefariError";
  readonly code: CefariErrorCode;
  readonly details: CefariIpcError;
  readonly command?: string;

  constructor(details: CefariIpcError, options: { command?: string } = {}) {
    super(errorMessage(details));
    this.code = details.code;
    this.details = details;
    this.command = options.command;
  }
}

export type CefariResult<T> =
  | { ok: true; value: T }
  | { ok: false; error: CefariError };

export function isCefariError(error: unknown): error is CefariError {
  return error instanceof CefariError;
}

export function unsupportedBridgeError(reason: string): CefariError {
  return new CefariError({
    code: "unsupported",
    details: {
      command: "bridge",
      reason,
    },
  });
}

export function unexpectedResultError(
  command: CefariIpcCommand,
  expected: string,
  actual: CefariIpcResult,
): CefariError {
  return new CefariError(
    {
      code: "invalidCommand",
      details: {
        message:
          `expected ${expected} result for ${command.command}, received ${actual.result}`,
      },
    },
    { command: command.command },
  );
}

function errorMessage(error: CefariIpcError): string {
  switch (error.code) {
    case "invalidCommand":
      return error.details.message;
    case "denied":
      return error.details.message;
    case "unknownCommand":
      return `unknown Cefari command: ${error.details.command}`;
    case "unsupported":
      return `unsupported Cefari command ${error.details.command}: ${error.details.reason}`;
  }
}
