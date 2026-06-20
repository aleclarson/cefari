import {
  createLogStore,
  formatLogEntry,
  getCefariLogDatabasePath,
  type LogEntry,
  type LogLevel,
  type LogQuery,
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

function toLogQuery(options: LogCliQueryOptions): LogQuery {
  return {
    afterId: options.afterId,
    beforeId: options.beforeId,
    debugScope: options.debugScope,
    grep: options.grep,
    level: options.level ?? "info",
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
