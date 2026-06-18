import { cefari } from "cefari/app";

export default function App() {
  const bridgeState = cefari.isAvailable() ? "connected" : "unavailable";

  return (
    <main className="app-shell">
      <section className="hero">
        <p className="eyebrow">Cefari + Vite + React</p>
        <h1>Hello from Cefari</h1>
        <p>
          This template runs Vite as the frontend dev server while Cefari
          orchestrates the desktop shell.
        </p>
        <p className="status">Bridge status: {bridgeState}</p>
      </section>
    </main>
  );
}
