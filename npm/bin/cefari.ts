#!/usr/bin/env -S deno run -A

import { runCefariCli } from "../src/index.js";

await runCefariCli(process.argv);
