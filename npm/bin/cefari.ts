#!/usr/bin/env -S deno run --allow-read --allow-write --allow-run --allow-env --allow-net --allow-ffi --allow-sys

import { runCefariCli } from "../src/index.js";

await runCefariCli(process.argv);
