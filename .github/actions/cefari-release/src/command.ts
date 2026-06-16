export interface CommandOptions {
  cwd?: string;
}

export type CommandRunner = (args: string[], options?: CommandOptions) => Promise<void>;

export async function runCommand(
  args: string[],
  dryRun: boolean,
  options: CommandOptions = {},
): Promise<void> {
  console.log(`+ ${args.map(quoteArg).join(" ")}`);
  if (dryRun) return;

  const command = new Deno.Command(args[0], {
    args: args.slice(1),
    cwd: options.cwd,
    stdin: "inherit",
    stdout: "inherit",
    stderr: "inherit",
  });
  const child = command.spawn();
  const status = await child.status;
  if (!status.success) {
    throw new Error(`${args[0]} exited with status ${status.code}`);
  }
}

export async function commandSucceeds(args: string[]): Promise<boolean> {
  const command = new Deno.Command(args[0], {
    args: args.slice(1),
    stdin: "null",
    stdout: "null",
    stderr: "null",
  });
  const status = await command.output().catch(() => undefined);
  return status?.success === true;
}

export async function validateCommandAvailable(
  commandName: string,
  dryRun: boolean,
): Promise<void> {
  if (dryRun) return;
  const available = await commandAvailable(commandName);
  if (!available) {
    throw new Error(`${commandName} is required but was not found`);
  }
}

async function commandAvailable(commandName: string): Promise<boolean> {
  if (commandName.includes("/") || commandName.includes("\\")) {
    return await Deno.stat(commandName).then((info) => info.isFile, () => false);
  }
  if (Deno.build.os === "windows") {
    return await commandSucceeds(["where", commandName]);
  }
  return await commandSucceeds(["sh", "-c", 'command -v "$1" >/dev/null 2>&1', "sh", commandName]);
}

export function quoteArg(arg: string): string {
  if (/^[A-Za-z0-9_./:@%+=,-]+$/.test(arg)) {
    return arg;
  }
  return `'${arg.replaceAll("'", `'"'"'`)}'`;
}
