import {
  createLogStore,
  formatLogEntry,
  getCefariLogDatabasePath,
  type LogEntry,
  type LogExportRecord,
  type LogLevel,
  type LogQuery,
  type SentryLogRecord,
  toSentryLogRecord,
} from "./logs.js";

export type LogOutputFormat = "text" | "json";

export type LogCliQueryOptions = {
  afterId?: number;
  beforeId?: number;
  debugScope?: string;
  grep?: string;
  json?: boolean;
  level?: LogLevel;
  limit?: number;
  properties?: string[];
  regex?: string;
  scope?: string;
  since?: string;
};

export type LogTailOptions = LogCliQueryOptions & {
  once?: boolean;
  pollMs?: number;
};

export type LogExportSentrySink = {
  export: (records: LogExportRecord[]) => Promise<SentryLogRecord[]>;
  flush: (timeout?: number) => Promise<boolean>;
};

export type LogExportSentryOptions = LogCliQueryOptions & {
  batchSize?: number;
  cursor?: string;
  databasePath?: string;
  dryRun?: boolean;
  dsn?: string;
  environment?: string;
  once?: boolean;
  pollMs?: number;
  release?: string;
  sampleRate?: number;
  sinkFactory?: (options: {
    dsn: string;
    environment?: string;
    release?: string;
    sampleRate?: number;
  }) => Promise<LogExportSentrySink> | LogExportSentrySink;
};

export function runLogsPath(): void {
  console.log(getCefariLogDatabasePath());
}

export function runLogsPage(options: LogCliQueryOptions): void {
  const store = createLogStore();
  try {
    const entries = store.query(toLogQuery(options));
    writeOutput(entries, options.json === true ? "json" : "text");
  } finally {
    store.close();
  }
}

export async function runLogsTail(options: LogTailOptions): Promise<void> {
  let afterId = options.afterId;

  while (true) {
    const store = createLogStore();
    try {
      const entries = store.query(toLogQuery({
        ...options,
        afterId,
        beforeId: undefined,
      }));
      if (entries.length > 0) {
        writeOutput(entries, options.json === true ? "json" : "text");
      }
      const last = entries.at(-1);
      if (last !== undefined) {
        afterId = last.id;
      }
    } finally {
      store.close();
    }

    if (options.once === true) {
      return;
    }

    await delay(options.pollMs ?? 1000);
  }
}

export async function runLogsExportSentry(options: LogExportSentryOptions): Promise<void> {
  const dryRun = options.dryRun === true;
  const cursor = options.cursor ?? "sentry";
  const batchSize = options.batchSize ?? 100;
  const dsn = options.dsn ?? process.env.SENTRY_DSN;
  let sink: LogExportSentrySink | undefined;

  if (batchSize < 1) {
    throw new Error("--batch-size must be greater than zero");
  }

  if (!dryRun && (dsn === undefined || dsn.trim() === "")) {
    throw new Error("logs export sentry requires --dsn or SENTRY_DSN");
  }

  while (true) {
    const store = createLogStore({ databasePath: options.databasePath });
    let records: LogExportRecord[] = [];
    try {
      records = store.exportBatch({
        exporter: cursor,
        limit: batchSize,
        query: toLogQuery(options, { defaultLevel: undefined }),
      });

      if (dryRun) {
        writeSentryRecords(records.map(toSentryLogRecord));
      } else if (records.length > 0) {
        sink = sink ?? await createSentrySink(options, dsn as string);
        await sink.export(records);
        const flushed = await sink.flush();
        if (flushed !== true) {
          throw new Error("Sentry log flush did not complete");
        }
        store.ackExport(cursor, records.at(-1)?.id ?? 0);
      }
    } finally {
      store.close();
    }

    if (dryRun || options.once === true) {
      return;
    }

    await delay(options.pollMs ?? 1000);
  }
}

export function runLogsExpand(id: string, options: { json?: boolean } = {}): void {
  const store = createLogStore();
  try {
    const value = store.expand(id);
    if (value === null) {
      throw new Error(`collapsed log value not found: ${id}`);
    }

    if (options.json === false) {
      console.log(String(value.body));
      return;
    }

    console.log(JSON.stringify(value, null, 2));
  } finally {
    store.close();
  }
}

function toLogQuery(options: LogCliQueryOptions, defaults: { defaultLevel?: LogLevel } = { defaultLevel: "info" }): LogQuery {
  return {
    afterId: options.afterId,
    beforeId: options.beforeId,
    debugScope: options.debugScope,
    grep: options.grep,
    level: options.level ?? defaults.defaultLevel,
    limit: options.limit,
    properties: parsePropertyFilters(options.properties ?? []),
    regex: options.regex,
    scope: options.scope,
    since: options.since === undefined ? undefined : normalizeSince(options.since),
  };
}

function parsePropertyFilters(filters: string[]): Record<string, string> {
  return Object.fromEntries(filters.map((filter) => {
    const separator = filter.indexOf("=");
    if (separator <= 0) {
      throw new Error(`log property filters must use key=value syntax: ${filter}`);
    }

    return [filter.slice(0, separator), filter.slice(separator + 1)];
  }));
}

function normalizeSince(value: string): string {
  const duration = /^(\d+)([smhd])$/.exec(value);
  if (duration === null) {
    return value;
  }

  const amount = Number(duration[1]);
  const unit = duration[2];
  const multiplier = unit === "s" ? 1000
    : unit === "m" ? 60 * 1000
    : unit === "h" ? 60 * 60 * 1000
    : 24 * 60 * 60 * 1000;
  return new Date(Date.now() - amount * multiplier).toISOString();
}

async function createSentrySink(options: LogExportSentryOptions, dsn: string): Promise<LogExportSentrySink> {
  if (options.sinkFactory !== undefined) {
    return await options.sinkFactory({
      dsn,
      environment: options.environment ?? process.env.SENTRY_ENVIRONMENT,
      release: options.release ?? process.env.SENTRY_RELEASE,
      sampleRate: options.sampleRate,
    });
  }

  const { createSentryLogSink } = await import("./sentry-logs.js");
  return createSentryLogSink({
    dsn,
    environment: options.environment ?? process.env.SENTRY_ENVIRONMENT,
    release: options.release ?? process.env.SENTRY_RELEASE,
    sampleRate: options.sampleRate,
  });
}

function writeSentryRecords(records: SentryLogRecord[]): void {
  console.log(JSON.stringify(records, null, 2));
}

function writeOutput(entries: LogEntry[], format: LogOutputFormat): void {
  if (format === "json") {
    console.log(JSON.stringify(entries, null, 2));
    return;
  }

  for (const entry of entries) {
    console.log(formatLogEntry(entry));
  }
}

function delay(milliseconds: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, milliseconds));
}
