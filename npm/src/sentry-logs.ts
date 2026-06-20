import * as Sentry from "@sentry/node";
import type { NodeOptions } from "@sentry/node";

import {
  toSentryLogRecord,
  type LogEntry,
  type LogExportRecord,
  type LogProperties,
  type SentryLogLevel,
  type SentryLogRecord,
} from "./logs.js";

export type SentryLogLogger = Record<
  SentryLogLevel,
  (message: string, attributes?: LogProperties) => void
>;

export type SentryLogClient = {
  init: (options: NodeOptions & { enableLogs: true }) => void;
  logger: SentryLogLogger;
  flush?: (timeout?: number) => Promise<boolean>;
};

export type SentryLogSinkOptions = {
  client?: SentryLogClient;
  dsn: string;
  environment?: string;
  release?: string;
  sampleRate?: NodeOptions["sampleRate"];
  beforeSendLog?: NodeOptions["beforeSendLog"];
};

export type SentryLogSink = {
  export: (records: Array<LogEntry | LogExportRecord>) => Promise<SentryLogRecord[]>;
  flush: (timeout?: number) => Promise<boolean>;
};

export function createSentryLogSink(options: SentryLogSinkOptions): SentryLogSink {
  const client = options.client ?? (Sentry as unknown as SentryLogClient);
  client.init({
    dsn: options.dsn,
    environment: options.environment,
    release: options.release,
    sampleRate: options.sampleRate,
    beforeSendLog: options.beforeSendLog,
    enableLogs: true,
  });

  return {
    async export(records) {
      const sentryRecords = records.map(toSentryLogRecord);
      for (const record of sentryRecords) {
        client.logger[record.level](record.message, record.attributes);
      }
      return sentryRecords;
    },
    async flush(timeout) {
      return await (client.flush?.(timeout) ?? Promise.resolve(true));
    },
  };
}
