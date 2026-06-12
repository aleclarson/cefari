console.log("cefari smoke daemon started");

setInterval(() => {
  console.log(`cefari smoke daemon heartbeat ${new Date().toISOString()}`);
}, 60_000);
