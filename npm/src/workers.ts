import { mkdir, writeFile } from "node:fs/promises";
import { dirname, relative, resolve, sep } from "node:path";
import type { ResolvedCefariConfig } from "./config.js";

export const WORKER_TYPES_PATH = ".cefari/workers.d.ts";

export async function generateWorkerRegistryTypes(config: ResolvedCefariConfig): Promise<string> {
  const outputPath = resolve(config.root, WORKER_TYPES_PATH);
  await mkdir(dirname(outputPath), { recursive: true });
  const contents = workerRegistryTypes(config, outputPath);
  await writeFile(outputPath, contents);
  return outputPath;
}

export function workerRegistryTypes(config: ResolvedCefariConfig, outputPath: string): string {
  const workers = Object.entries(config.workers);
  const imports = workers
    .map(([id, worker], index) => {
      const binding = workerBinding(index);
      const specifier = relativeModuleSpecifier(dirname(outputPath), resolve(config.root, worker.entry));
      return `import type ${binding} from ${JSON.stringify(specifier)};\n`;
    })
    .join("");
  const inferImport = workers.length === 0 ? "" : 'import type { InferCefariWorker } from "cefari/worker";\n';
  const registry = workers
    .map(([id], index) => `    ${JSON.stringify(id)}: InferCefariWorker<typeof ${workerBinding(index)}>;\n`)
    .join("");

  return `${imports}${inferImport}
declare module "cefari/app" {
  interface CefariWorkerRegistry {
${registry}  }
}

export {};
`;
}

function workerBinding(index: number): string {
  return `worker_${index}`;
}

function relativeModuleSpecifier(fromDirectory: string, toFile: string): string {
  const relativePath = relative(fromDirectory, toFile).split(sep).join("/");
  return relativePath.startsWith(".") ? relativePath : `./${relativePath}`;
}
