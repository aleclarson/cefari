import { defineWorker, runCefariWorker } from "cefari/worker";

type Init = { label: string };
type ComputeInput = { iterations: number };
type ComputeMessage = { phase: "started" | "progress" | "finished"; label: string; iteration?: number };
type ComputeOutput = { label: string; iterations: number; checksum: number; elapsedMs: number };

const worker = defineWorker((init: Init) => ({
  async compute(input: ComputeInput, context: { postMessage(message: ComputeMessage): Promise<void> }): Promise<ComputeOutput> {
    const started = performance.now();
    let checksum = 0;
    await context.postMessage({ phase: "started", label: init.label });

    for (let index = 0; index < input.iterations; index += 1) {
      checksum = (checksum + ((index * 2654435761) >>> 0)) >>> 0;
      if (index > 0 && index % Math.max(1, Math.floor(input.iterations / 4)) === 0) {
        await context.postMessage({ phase: "progress", label: init.label, iteration: index });
      }
    }

    await context.postMessage({ phase: "finished", label: init.label });
    return { label: init.label, iterations: input.iterations, checksum, elapsedMs: performance.now() - started };
  },
}));

if (import.meta.main) {
  Deno.exit(await runCefariWorker(worker));
}
