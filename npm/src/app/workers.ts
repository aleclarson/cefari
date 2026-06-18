import { on, type CefariEventMap } from "./events.ts";
import type { WorkerState } from "./ipc.ts";
import { invokeWorker } from "./results.ts";
import type { Unsubscribe } from "./transport.ts";

export interface CefariWorkerRegistry {}

export type CefariWorkerName = Extract<keyof CefariWorkerRegistry, string>;

export type CefariWorkerInput<Name extends CefariWorkerName> =
  CefariWorkerRegistry[Name]["input"];

export type CefariWorkerOutput<Name extends CefariWorkerName> =
  CefariWorkerRegistry[Name]["output"];

export type CefariWorkerMessage<Name extends CefariWorkerName> =
  CefariWorkerRegistry[Name]["message"];

export type CefariWorkerHandle<Name extends CefariWorkerName> = {
  id: string;
  worker: Name;
  status: "running" | "exited";
  terminate(): Promise<void>;
  onMessage(handler: (message: CefariWorkerMessage<Name>) => void): Unsubscribe;
  onExit(handler: (event: CefariWorkerExitEvent<Name>) => void): Unsubscribe;
  onError(handler: (event: CefariWorkerErrorEvent<Name>) => void): Unsubscribe;
};

export type CefariWorkerExitEvent<Name extends CefariWorkerName> = Omit<
  CefariEventMap["worker.exited"],
  "worker"
> & {
  worker: Name;
};

export type CefariWorkerErrorEvent<Name extends CefariWorkerName> = Omit<
  CefariEventMap["worker.error"],
  "worker"
> & {
  worker: Name;
};

export type WorkersApi = {
  spawn<Name extends CefariWorkerName>(
    worker: Name,
    input: CefariWorkerInput<Name>,
  ): Promise<CefariWorkerHandle<Name>>;
  terminate(id: string): Promise<void>;
  list(): Promise<WorkerState[]>;
};

export const workers: WorkersApi = {
  async spawn(worker, input) {
    const result = await invokeWorker({
      command: "worker",
      payload: {
        worker: "spawn",
        payload: {
          worker,
          inputJson: JSON.stringify(input),
        },
      },
    });
    if (result.result !== "spawned") {
      throw new Error(`expected spawned worker result, received ${result.result}`);
    }
    return workerHandle(worker, result.payload.id, result.payload.status);
  },
  async terminate(id) {
    const result = await invokeWorker({
      command: "worker",
      payload: {
        worker: "terminate",
        payload: { id },
      },
    });
    if (result.result !== "terminated") {
      throw new Error(`expected terminated worker result, received ${result.result}`);
    }
  },
  async list() {
    const result = await invokeWorker({
      command: "worker",
      payload: { worker: "list" },
    });
    if (result.result !== "list") {
      throw new Error(`expected worker list result, received ${result.result}`);
    }
    return result.payload.workers;
  },
};

function workerHandle<Name extends CefariWorkerName>(
  worker: Name,
  id: string,
  status: "running" | "exited",
): CefariWorkerHandle<Name> {
  return {
    id,
    worker,
    status,
    terminate: () => workers.terminate(id),
    onMessage(handler) {
      return on("worker.message", (event) => {
        if (event.id === id && event.worker === worker) {
          handler(JSON.parse(event.messageJson) as CefariWorkerMessage<Name>);
        }
      });
    },
    onExit(handler) {
      return on("worker.exited", (event) => {
        if (event.id === id && event.worker === worker) {
          handler(event as CefariWorkerExitEvent<Name>);
        }
      });
    },
    onError(handler) {
      return on("worker.error", (event) => {
        if (event.id === id && event.worker === worker) {
          handler(event as CefariWorkerErrorEvent<Name>);
        }
      });
    },
  };
}
