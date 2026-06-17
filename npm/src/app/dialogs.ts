import { invokeDialog } from "./results.ts";
import type {
  DialogDefaultDirectory,
  DialogFilter,
  DialogModality,
  DialogRequest,
  DialogResult,
  DialogSelectedPath,
} from "./ipc.ts";

export type {
  DialogDefaultDirectory,
  DialogFilter,
  DialogModality,
  DialogSelectedPath,
};

export type DialogOptions = {
  title?: string | null;
  filters?: DialogFilter[];
  defaultDirectory?: DialogDefaultDirectory | null;
  defaultName?: string | null;
  modality?: DialogModality | null;
  canCreateDirectories?: boolean | null;
};

export type DialogOutcome<T> =
  | { canceled: true }
  | { canceled: false; value: T };

export type DialogsApi = {
  openFile(options?: DialogOptions): Promise<DialogOutcome<DialogSelectedPath>>;
  openFiles(
    options?: DialogOptions,
  ): Promise<DialogOutcome<DialogSelectedPath[]>>;
  chooseFolder(
    options?: DialogOptions,
  ): Promise<DialogOutcome<DialogSelectedPath>>;
  chooseFolders(
    options?: DialogOptions,
  ): Promise<DialogOutcome<DialogSelectedPath[]>>;
  saveFile(options?: DialogOptions): Promise<DialogOutcome<DialogSelectedPath>>;
};

async function openFile(
  options?: DialogOptions,
): Promise<DialogOutcome<DialogSelectedPath>> {
  return singleSelection(
    await invokeDialogCommand("openFile", normalizeOptions(options)),
    "openFile",
  );
}

async function openFiles(
  options?: DialogOptions,
): Promise<DialogOutcome<DialogSelectedPath[]>> {
  return multiSelection(
    await invokeDialogCommand("openFiles", normalizeOptions(options)),
  );
}

async function chooseFolder(
  options?: DialogOptions,
): Promise<DialogOutcome<DialogSelectedPath>> {
  return singleSelection(
    await invokeDialogCommand("chooseFolder", normalizeOptions(options)),
    "chooseFolder",
  );
}

async function chooseFolders(
  options?: DialogOptions,
): Promise<DialogOutcome<DialogSelectedPath[]>> {
  return multiSelection(
    await invokeDialogCommand("chooseFolders", normalizeOptions(options)),
  );
}

async function saveFile(
  options?: DialogOptions,
): Promise<DialogOutcome<DialogSelectedPath>> {
  return singleSelection(
    await invokeDialogCommand("saveFile", normalizeOptions(options)),
    "saveFile",
  );
}

function normalizeOptions(options: DialogOptions = {}): DialogRequest {
  return {
    title: options.title ?? null,
    filters: options.filters ?? [],
    defaultDirectory: options.defaultDirectory ?? null,
    defaultName: options.defaultName ?? null,
    modality: options.modality ?? "window",
    canCreateDirectories: options.canCreateDirectories ?? null,
  };
}

async function invokeDialogCommand(
  dialog: "openFile" | "openFiles" | "chooseFolder" | "chooseFolders" | "saveFile",
  payload: DialogRequest,
): Promise<DialogResult> {
  return await invokeDialog({
    command: "dialog",
    payload: { dialog, payload },
  });
}

function singleSelection(
  result: DialogResult,
  operation: string,
): DialogOutcome<DialogSelectedPath> {
  if (result.result === "canceled") return { canceled: true };
  const [selected] = result.payload.paths;
  if (!selected || result.payload.paths.length !== 1) {
    throw new Error(
      `Unexpected ${operation} dialog selection count: ${result.payload.paths.length}`,
    );
  }
  return { canceled: false, value: selected };
}

function multiSelection(
  result: DialogResult,
): DialogOutcome<DialogSelectedPath[]> {
  if (result.result === "canceled") return { canceled: true };
  return { canceled: false, value: result.payload.paths };
}

export const dialogs: Readonly<DialogsApi> = Object.freeze({
  openFile,
  openFiles,
  chooseFolder,
  chooseFolders,
  saveFile,
});
