# Cefari CLI

This package installs the `cefari` command through npm.

Use it with:

```bash
npm install -g cefari
cefari --help
```

The package depends on a platform-specific `@cefari/cli-*` package that bundles
the native Rust `cefari` binary and the matching `cefari-desktop` runtime.
