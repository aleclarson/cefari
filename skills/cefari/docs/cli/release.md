# Release Commands

Use release commands for signing, notarization, and update metadata. They are
thin wrappers around the signing tools used by Cefari release automation.

## `cefari codesign`

Code sign a packaged app artifact:

```bash
cefari codesign ARTIFACT [--platform PLATFORM] [--config PATH]
```

Arguments and options:

- `ARTIFACT`: artifact to sign.
- `--platform PLATFORM`: `macos`, `windows`, or `linux`. Defaults to the host
  platform.
- `--config PATH`: path to `sign.toml`.

macOS signing accepts `.app` bundles and `.dmg` artifacts. Linux signing passes
the artifact as an archive to the signing tool.

## `cefari notarize`

Notarize a signed macOS artifact:

```bash
cefari notarize ARTIFACT [--config PATH]
```

Arguments and options:

- `ARTIFACT`: macOS `.app` bundle or `.dmg`.
- `--config PATH`: path to `sign.toml`.

This command is macOS-oriented. Use it after signing when Apple notarization is
required for distribution.

## `cefari make-update`

Generate update signature and manifest artifacts:

```bash
cefari make-update ARCHIVE --url URL --version VERSION [OPTIONS]
```

Arguments and required options:

- `ARCHIVE`: release archive to sign for update installation.
- `--url URL`: public download URL for the release archive.
- `--version VERSION`: version advertised to the runtime updater.

Optional flags:

- `--target TARGET`: updater target key. Defaults to the host OS and arch, such
  as `macos-aarch64`, `windows-x86_64`, or `linux-x86_64`.
- `--format FORMAT`: `app`, `appimage`, `nsis`, or `wix`. Defaults from
  `--target`.
- `--key-env NAME`: environment variable read by the signing tool for the update
  signing key. Defaults to `UPDATE_SIGNING_KEY`.
- `--output-dir PATH`: output directory. Defaults to `dist/update`.

Outputs:

- `<archive-file-name>.sig`
- `update.json`

The generated `update.json` contains the advertised version, target platform,
package format, signature, and archive URL.
