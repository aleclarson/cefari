import { signal } from "@preact/signals";
import { h, render } from "preact";
import { features } from "./features/catalog.ts";
import type { FeatureCategory, FeatureModule } from "./features/types.ts";
import "./styles.css";

type RunState = "idle" | "running" | "passed" | "failed";
type FeatureRun = { state: RunState; output?: unknown; error?: string; logs: string[] };

const categories = ["All", ...Array.from(new Set(features.map((feature) => feature.category)))] as const;
const selectedCategory = signal<(typeof categories)[number]>("All");
const runs = signal<Record<string, FeatureRun>>(Object.fromEntries(features.map((feature) => [feature.id, { state: "idle", logs: [] }])));

function updateRun(id: string, patch: Partial<FeatureRun>) {
  runs.value = { ...runs.value, [id]: { ...runs.value[id], ...patch } };
}

async function runFeature(feature: FeatureModule) {
  updateRun(feature.id, { state: "running", output: undefined, error: undefined, logs: [] });
  try {
    const output = await feature.run({
      log(message, data) {
        const suffix = data === undefined ? "" : ` ${format(data)}`;
        updateRun(feature.id, { logs: [...runs.value[feature.id].logs, `${new Date().toLocaleTimeString()} ${message}${suffix}`] });
      },
    });
    updateRun(feature.id, { state: "passed", output });
  } catch (error) {
    updateRun(feature.id, { state: "failed", error: error instanceof Error ? error.message : String(error) });
  }
}

function format(value: unknown) {
  try { return JSON.stringify(value, null, 2); } catch { return String(value); }
}

function App() {
  const visible = selectedCategory.value === "All" ? features : features.filter((feature) => feature.category === selectedCategory.value);
  return <main>
    <h1>Cefari Playground</h1>
    <p>Feature modules are grouped by category and can be exercised individually.</p>
    <label>Category <select value={selectedCategory.value} onInput={(event) => selectedCategory.value = event.currentTarget.value as FeatureCategory | "All"}>{categories.map((category) => <option value={category}>{category}</option>)}</select></label>
    <hr />
    {visible.map((feature) => <FeatureCard feature={feature} run={runs.value[feature.id]} />)}
  </main>;
}

function FeatureCard({ feature, run }: { feature: FeatureModule; run: FeatureRun }) {
  return <section>
    <header>
      <h2>{feature.name}</h2>
      <p>{feature.category} · {run.state}</p>
    </header>
    <p>{feature.description}</p>
    <button disabled={run.state === "running"} onClick={() => runFeature(feature)}>{run.state === "running" ? "Running…" : "Run"}</button>
    {run.logs.length > 0 && <pre>{run.logs.join("\n")}</pre>}
    {run.error && <pre>Error: {run.error}</pre>}
    {run.output !== undefined && <pre>{format(run.output)}</pre>}
    <hr />
  </section>;
}

render(<App />, document.getElementById("app")!);
