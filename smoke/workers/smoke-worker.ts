import { defineWorker, runCefariWorker } from "cefari/worker";

interface SmokeWorkerInput {
  inputPath: string;
  outputPath: string;
  deniedPath: string;
  holdMs?: number;
}

interface SmokeWorkerOutput {
  uppercased: string;
  denied: boolean;
}

type SmokeWorkerMessage =
  | { phase: "started" }
  | { phase: "permission-denied"; message: string }
  | { phase: "holding"; holdMs: number };

const worker = defineWorker((_init: null) => ({
  async transform(input: SmokeWorkerInput, context: { postMessage(message: SmokeWorkerMessage): Promise<void> }): Promise<SmokeWorkerOutput> {
    console.error("cefari smoke worker transform started");
    await context.postMessage({ phase: "started" });

    const contents = await Deno.readTextFile(input.inputPath);
    const uppercased = contents.toUpperCase();
    await Deno.writeTextFile(input.outputPath, uppercased);

    let denied = false;
    try {
      await Deno.readTextFile(input.deniedPath);
    } catch (error) {
      denied = error instanceof Deno.errors.PermissionDenied;
      await context.postMessage({
        phase: "permission-denied",
        message: error instanceof Error ? error.message : String(error),
      });
    }

    if (input.holdMs !== undefined) {
      await context.postMessage({ phase: "holding", holdMs: input.holdMs });
      await new Promise((resolve) => setTimeout(resolve, input.holdMs));
    }

    return { uppercased, denied };
  },
}));

if (import.meta.main) {
  Deno.exit(await runCefariWorker(worker));
}
