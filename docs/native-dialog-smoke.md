# Native Dialog Smoke Checklist

Use this checklist on desktop platforms before relying on native file and folder
dialogs in release builds. Automated tests must not open native dialogs, so
these checks are intentionally human-run.

## Test Surface

- Open file:
  - Trigger `cefari.dialogs.openFile()`.
  - Select an existing file.
  - Expected: the result is not canceled and contains one selected file path.
- Open files:
  - Trigger `cefari.dialogs.openFiles()`.
  - Select multiple existing files.
  - Expected: the result is not canceled and contains every selected file path.
- Choose folder:
  - Trigger `cefari.dialogs.chooseFolder()`.
  - Select an existing folder.
  - Expected: the result is not canceled and contains one selected directory
    path.
- Choose folders:
  - Trigger `cefari.dialogs.chooseFolders()`.
  - Select multiple existing folders when the platform dialog allows it.
  - Expected: the result is not canceled and contains the selected directories,
    or the platform clearly limits folder multi-selection.
- Save file:
  - Trigger `cefari.dialogs.saveFile({ defaultName: "report.txt" })`.
  - Confirm a save path.
  - Expected: the result contains the selected path and no file is written by
    Cefari.
- Cancellation:
  - Cancel each dialog variant.
  - Expected: each call resolves with `{ canceled: true }`.
- Filters:
  - Trigger a file dialog with filters such as `{ name: "Images", extensions:
    ["png", "jpg"] }`.
  - Expected: the native picker shows platform-appropriate filtering.
- Default directory:
  - Trigger a dialog with a native default directory.
  - Trigger a dialog with an app-data default directory.
  - Expected: the dialog starts in the requested directory when the platform
    honors default directories.
- Default name:
  - Trigger a save dialog with `defaultName`.
  - Expected: the native picker pre-fills the name when the platform supports
    it.
- Modality:
  - Trigger a dialog from the main window.
  - Expected: the dialog is associated with the Cefari window and does not open
    behind it.
- Directory creation:
  - Trigger a dialog with `canCreateDirectories: true`.
  - Expected: macOS exposes directory creation where supported. Other platforms
    may ignore the option.

## macOS Notes

- File filters in save dialogs can affect extension validation.
- macOS may append the first filter extension when the user omits an extension.
- `canCreateDirectories` is expected to be honored on macOS.

## Linux Notes

- Cefari uses the `rfd` default Linux backend, which prefers XDG Desktop Portal.
- The desktop environment must provide a compatible portal backend such as GTK,
  GNOME, or KDE.
- Portal and GTK behavior can differ for parent windows, filters, and
  multi-folder selection.

## Windows Notes

- Verify dialogs are owned by the main Cefari window.
- Verify extension filters and default names appear in the Windows picker.

## Security Checks

- Select a native path and then try passing that absolute path to `cefari.fs`.
  Expected: `cefari.fs` continues to reject absolute paths.
- Use `{ kind: "appData", path: "../secret" }` as a default directory.
  Expected: Cefari rejects the request before opening a native dialog.
- Use invalid filters such as `.txt`, `*.txt`, or `nested/txt`.
  Expected: Cefari rejects the request before opening a native dialog.
