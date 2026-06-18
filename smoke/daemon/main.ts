console.error("cefari smoke daemon started");

await Deno.stdin.readable.pipeTo(Deno.stdout.writable);
