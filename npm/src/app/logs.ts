import type { CefariResult } from "./errors.ts";
import { invokeEmpty, tryInvokeEmpty } from "./results.ts";

export type LogLevel = "debug" | "error" | "info" | "log" | "warn";
export type LogProperties = Record<string, unknown>;

export type LogsApi = {
  debug(message: string, properties?: LogProperties): Promise<void>;
  error(message: string, properties?: LogProperties): Promise<void>;
  info(message: string, properties?: LogProperties): Promise<void>;
  log(message: string, properties?: LogProperties): Promise<void>;
  tryWrite(
    level: LogLevel,
    message: string,
    properties?: LogProperties,
  ): Promise<CefariResult<void>>;
  warn(message: string, properties?: LogProperties): Promise<void>;
  write(
    level: LogLevel,
    message: string,
    properties?: LogProperties,
  ): Promise<void>;
};

async function write(
  level: LogLevel,
  message: string,
  properties: LogProperties = {},
): Promise<void> {
  await invokeEmpty({
    command: "log",
    payload: {
      level,
      message,
      propertiesJson: JSON.stringify(properties),
    },
  });
}

async function tryWrite(
  level: LogLevel,
  message: string,
  properties: LogProperties = {},
): Promise<CefariResult<void>> {
  return await tryInvokeEmpty({
    command: "log",
    payload: {
      level,
      message,
      propertiesJson: JSON.stringify(properties),
    },
  });
}

export const logs: Readonly<LogsApi> = Object.freeze({
  debug: (message, properties) => write("debug", message, properties),
  error: (message, properties) => write("error", message, properties),
  info: (message, properties) => write("info", message, properties),
  log: (message, properties) => write("log", message, properties),
  tryWrite,
  warn: (message, properties) => write("warn", message, properties),
  write,
});
