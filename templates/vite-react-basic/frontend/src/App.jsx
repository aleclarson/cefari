import { useEffect, useState } from "react";

import { invokeCefari } from "./cefari.ts";

export default function App() {
  const [bridgeState, setBridgeState] = useState("checking");

  useEffect(() => {
    let active = true;

    invokeCefari({ command: "updateState" }).then((response) => {
      if (!active) return;

      if (response.outcome.status === "ok") {
        setBridgeState(response.outcome.payload.result);
      } else {
        setBridgeState(response.outcome.payload.code);
      }
    });

    return () => {
      active = false;
    };
  }, []);

  return (
    <main className="app-shell">
      <section className="hero">
        <p className="eyebrow">Cefari + Vite + React</p>
        <h1>Hello from Cefari</h1>
        <p>
          This template runs Vite as the frontend dev server while Cefari
          orchestrates the desktop shell and Deno daemon.
        </p>
        <p className="status">Bridge status: {bridgeState}</p>
      </section>
    </main>
  );
}
