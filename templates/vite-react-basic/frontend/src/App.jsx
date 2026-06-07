import { useEffect, useState } from "react";

import { cefari, isCefariError } from "@cefari/app";

export default function App() {
  const [bridgeState, setBridgeState] = useState("checking");

  useEffect(() => {
    let active = true;

    cefari.updates.state()
      .then((state) => {
        if (active) setBridgeState(state.state);
      })
      .catch((error) => {
        if (active) {
          setBridgeState(isCefariError(error) ? error.code : "error");
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
