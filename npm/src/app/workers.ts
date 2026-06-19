import { on, type CefariEventMap } from "./events.ts";
import type { WorkerState } from "./ipc.ts";
import { invokeWorker } from "./results.ts";
import type { Unsubscribe } from "./transport.ts";

export interface CefariWorkerRegistry {}

export type CefariWorkerName = Extract<keyof CefariWorkerRegistry, string>;

export type CefariWorkerInit<Name extends CefariWorkerName> =
  CefariWorkerRegistry[Name] extends { init: infer Init } ? Init : never;

export type CefariWorkerMethodName<Name extends CefariWorkerName> =
  CefariWorkerRegistry[Name] extends { methods: infer Methods }
    ? Extract<keyof Methods, string>
    : never;

export type CefariWorkerMethodContract<
  Name extends CefariWorkerName,
  Method extends CefariWorkerMethodName<Name>,
> = CefariWorkerRegistry[Name] extends { methods: infer Methods }
  ? Method extends keyof Methods
    ? Methods[Method]
    : never
  : never;

export type CefariWorkerMethodInput<
  Name extends CefariWorkerName,
  Method extends CefariWorkerMethodName<Name>,
> = CefariWorkerMethodContract<Name, Method> extends { input: infer Input } ? Input : never;

export type CefariWorkerMethodOutput<
  Name extends CefariWorkerName,
  Method extends CefariWorkerMethodName<Name>,
> = CefariWorkerMethodContract<Name, Method> extends { output: infer Output } ? Output : never;

export type CefariWorkerMethodMessage<
  Name extends CefariWorkerName,
  Method extends CefariWorkerMethodName<Name>,
> = CefariWorkerMethodContract<Name, Method> extends { message: infer Message } ? Message : never;

export type CefariWorkerMessage<Name extends CefariWorkerName> = {
  [Method in CefariWorkerMethodName<Name>]: CefariWorkerMethodMessage<Name, Method>;
}[CefariWorkerMethodName<Name>];

export type CefariWorkerHandle<Name extends CefariWorkerName> = {
  id: string;
  worker: Name;
  status: "running" | "exited";
  invoke<Method extends CefariWorkerMethodName<Name>>(
    method: Method,
    input: CefariWorkerMethodInput<Name, Method>,
  ): Promise<CefariWorkerMethodOutput<Name, Method>>;
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
    init: CefariWorkerInit<Name>,
  ): Promise<CefariWorkerHandle<Name>>;
  run<Name extends CefariWorkerName, Method extends CefariWorkerMethodName<Name>>(
    worker: Name,
    init: CefariWorkerInit<Name>,
    method: Method,
    input: CefariWorkerMethodInput<Name, Method>,
  ): Promise<CefariWorkerMethodOutput<Name, Method>>;
  terminate(id: string): Promise<void>;
  list(): Promise<WorkerState[]>;
};

export const workers: WorkersApi = {
  async spawn(worker, init) {
    const result = await invokeWorker({
      command: "worker",
      payload: {
        worker: "spawn",
        payload: {
          worker,
          inputJson: JSON.stringify(init),
        },
      },
    });
    if (result.result !== "spawned") {
      throw new Error(`expected spawned worker result, received ${result.result}`);
    }
    return workerHandle(worker, result.payload.id, result.payload.status);
  },
  async run(worker, init, method, input) {
    const handle = await workers.spawn(worker, init);
    try {
      return await handle.invoke(method, input);
    } finally {
      await handle.terminate();
    }
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
    async invoke(method, input) {
      const result = await invokeWorker({
        command: "worker",
        payload: {
          worker: "invoke",
          payload: {
            id,
            method,
            inputJson: JSON.stringify(input),
          },
        },
      });
      if (result.result !== "invoked") {
        throw new Error(`expected invoked worker result, received ${result.result}`);
      }
      return JSON.parse(result.payload.outputJson) as CefariWorkerMethodOutput<Name, typeof method>;
    },
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
