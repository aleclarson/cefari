import { on } from "./events.ts";
import { invokeServiceStatus } from "./results.ts";
import type { ServiceStatusResult } from "./ipc.ts";
import type { Unsubscribe } from "./transport.ts";

export type ServiceApi = {
  status(): Promise<ServiceStatusResult>;
  onStatusChanged(handler: (status: ServiceStatusResult) => void): Unsubscribe;
};

export const service: ServiceApi = {
  status: (): Promise<ServiceStatusResult> =>
    invokeServiceStatus({ command: "serviceStatus" }),
  onStatusChanged: (
    handler: (status: ServiceStatusResult) => void,
  ): Unsubscribe => on("serviceStatusChanged", handler),
};
