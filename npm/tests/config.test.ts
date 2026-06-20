import assert from "node:assert/strict";
import { mkdtemp, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import test from "node:test";
import {
  deepLinks,
  loadCefariConfig,
  toSerializableProjectConfig,
  tray,
} from "../src/index.js";

const testDir = dirname(fileURLToPath(import.meta.url));
const configApi = pathToFileURL(resolve(testDir, "../src/index.js")).href;

async function projectWithConfig(source: string): Promise<string> {
  const root = await mkdtemp(join(tmpdir(), "cefari-config-"));
  await writeFile(
    join(root, "cefari.config.ts"),
    source.replaceAll("__CEFARI_CONFIG_API__", configApi),
  );
  return root;
}

function baseConfig(extra = ""): string {
  return `import { defineConfig } from "__CEFARI_CONFIG_API__";

export default defineConfig({
  app: {
    projectName: "example-app",
    name: "Example App",
    identifier: "dev.cefari.example",
  },
  ${extra}
  daemon: {
    entry: "daemon/main.ts",
  },
  package: {
    productName: "Example App",
    version: "0.1.0",
  },
});
`;
}

test("loads an object config", async () => {
  const root = await projectWithConfig(baseConfig());

  const config = await loadCefariConfig({
    root,
    command: "build",
    mode: "production",
  });

  assert.equal(config.root, root);
  assert.equal(config.app.projectName, "example-app");
  assert.ok(config.daemon);
  assert.equal(config.daemon.entry, "daemon/main.ts");
  assert.deepEqual(config.targets.desktop.daemon, { entry: "daemon/main.ts" });
  assert.equal(config.package.version, "0.1.0");
  assert.deepEqual(config.workers, {});
});

test("loads a config without a daemon", async () => {
  const root = await projectWithConfig(
    `import { defineConfig } from "__CEFARI_CONFIG_API__";

export default defineConfig({
  app: {
    projectName: "example-app",
    name: "Example App",
    identifier: "dev.cefari.example",
  },
  package: {
    productName: "Example App",
    version: "0.1.0",
  },
});
`,
  );

  const config = await loadCefariConfig({ root });
  const serializable = toSerializableProjectConfig(config);

  assert.equal(config.daemon, undefined);
  assert.equal(Object.hasOwn(serializable, "daemon"), false);
  assert.deepEqual(serializable.targets.desktop, { capabilities: [] });
});

test("loads target-aware config", async () => {
  const root = await projectWithConfig(
    `import { deepLinks, defineConfig, tray } from "__CEFARI_CONFIG_API__";

export default defineConfig({
  app: {
    projectName: "target-app",
    name: "Target App",
    identifier: "dev.cefari.target",
  },
  capabilities: [
    deepLinks({ schemes: ["targetapp"] }),
  ],
  targets: {
    desktop: {
      capabilities: [
        tray({ icon: "assets/tray.png" }),
      ],
      daemon: {
        entry: "desktop/daemon.ts",
      },
    },
    ios: {
      bundleId: "dev.cefari.target.ios",
      permissions: ["notifications"],
    },
    android: {
      applicationId: "dev.cefari.target.android",
      permissions: ["notifications"],
    },
  },
  daemon: {
    entry: "daemon/main.ts",
  },
  package: {
    productName: "Target App",
    version: "0.1.0",
  },
});
`,
  );

  const config = await loadCefariConfig({ root });
  const serializable = toSerializableProjectConfig(config);

  assert.deepEqual(config.capabilities, [{
    type: "tray",
    icon: "assets/tray.png",
  }]);
  assert.deepEqual(config.daemon, { entry: "desktop/daemon.ts" });
  assert.deepEqual(config.targets.ios, {
    bundleId: "dev.cefari.target.ios",
    permissions: ["notifications"],
  });
  assert.deepEqual(config.targets.android, {
    applicationId: "dev.cefari.target.android",
    permissions: ["notifications"],
  });
  assert.deepEqual(serializable.targets.desktop.daemon, {
    entry: "desktop/daemon.ts",
  });
});

test("loads a factory config with Cefari context", async () => {
  const root = await projectWithConfig(
    `import { defineConfig } from "__CEFARI_CONFIG_API__";

export default defineConfig((context) => ({
  app: {
    projectName: context.mode === "production" ? "prod-app" : "dev-app",
    name: context.command,
    identifier: "dev.cefari.factory",
  },
  vite: {
    root: context.packageCommand === "release" ? "release-ui" : "frontend",
  },
  daemon: {
    entry: "daemon/main.ts",
  },
  package: {
    productName: "Factory App",
    version: "0.1.0",
  },
}));
`,
  );

  const config = await loadCefariConfig({
    root,
    command: "package",
    packageCommand: "release",
    mode: "production",
  });

  assert.equal(config.app.projectName, "prod-app");
  assert.equal(config.app.name, "package");
  assert.equal(config.vite.root, "release-ui");
});

test("applies Vite defaults", async () => {
  const root = await projectWithConfig(baseConfig());

  const config = await loadCefariConfig({ root });

  assert.deepEqual(config.browser, {
    webgpu: false,
  });
  assert.deepEqual(config.vite, {
    root: "frontend",
    devPort: 5173,
  });
});

test("supports browser WebGPU opt-in", async () => {
  const root = await projectWithConfig(
    baseConfig(`browser: { webgpu: true },`),
  );

  const config = await loadCefariConfig({ root });
  const serializable = toSerializableProjectConfig(config);

  assert.deepEqual(config.browser, {
    webgpu: true,
  });
  assert.deepEqual(serializable.browser, {
    webgpu: true,
  });
});

test("supports tray capability builder", async () => {
  const root = await projectWithConfig(
    `import { defineConfig, tray } from "__CEFARI_CONFIG_API__";

export default defineConfig({
  app: {
    projectName: "tray-app",
    name: "Tray App",
    identifier: "dev.cefari.tray",
  },
  capabilities: [
    tray({ icon: "assets/tray.png" }),
  ],
  daemon: {
    entry: "daemon/main.ts",
  },
  package: {
    productName: "Tray App",
    version: "0.1.0",
  },
});
`,
  );

  const config = await loadCefariConfig({ root });

  assert.deepEqual(config.capabilities, [{
    type: "tray",
    icon: "assets/tray.png",
  }]);
  assert.deepEqual(tray({ icon: "assets/tray.png" }), {
    type: "tray",
    icon: "assets/tray.png",
  });
});

test("supports deep links capability builder", async () => {
  const root = await projectWithConfig(
    `import { deepLinks, defineConfig } from "__CEFARI_CONFIG_API__";

export default defineConfig({
  app: {
    projectName: "links-app",
    name: "Links App",
    identifier: "dev.cefari.links",
  },
  capabilities: [
    deepLinks({ schemes: ["myapp", "myapp+dev"] }),
  ],
  daemon: {
    entry: "daemon/main.ts",
  },
  package: {
    productName: "Links App",
    version: "0.1.0",
  },
});
`,
  );

  const config = await loadCefariConfig({ root });

  assert.deepEqual(config.capabilities, [{
    type: "deepLinks",
    schemes: ["myapp", "myapp+dev"],
  }]);
  assert.deepEqual(deepLinks({ schemes: ["myapp"] }), {
    type: "deepLinks",
    schemes: ["myapp"],
  });
});

test("normalizes configured workers", async () => {
  const root = await projectWithConfig(baseConfig(`workers: {
    thumbnailer: {
      entry: "workers/thumbnailer.ts",
      permissions: {
        read: ["$appData/uploads"],
        write: ["$appData/cache"],
        net: "none",
      },
    },
  },`));

  const config = await loadCefariConfig({ root });

  assert.deepEqual(config.workers, {
    thumbnailer: {
      entry: "workers/thumbnailer.ts",
      permissions: {
        read: ["$appData/uploads"],
        write: ["$appData/cache"],
        net: "none",
        env: "none",
        run: "none",
      },
    },
  });
});

test("rejects invalid worker config", async () => {
  await assert.rejects(
    loadCefariConfig({
      root: await projectWithConfig(baseConfig(`workers: {
        Thumbnailer: {
          entry: "workers/thumbnailer.ts",
          permissions: {},
        },
      },`)),
    }),
    /workers\.Thumbnailer must use an id matching/,
  );
  await assert.rejects(
    loadCefariConfig({
      root: await projectWithConfig(baseConfig(`workers: {
        thumbnailer: {
          entry: "../workers/thumbnailer.ts",
          permissions: {},
        },
      },`)),
    }),
    /workers\.thumbnailer\.entry must be a relative path inside the project/,
  );
  await assert.rejects(
    loadCefariConfig({
      root: await projectWithConfig(baseConfig(`workers: {
        thumbnailer: {
          entry: "workers/thumbnailer.ts",
        },
      },`)),
    }),
    /workers\.thumbnailer\.permissions must be an object/,
  );
  await assert.rejects(
    loadCefariConfig({
      root: await projectWithConfig(baseConfig(`workers: {
        thumbnailer: {
          entry: "workers/thumbnailer.ts",
          permissions: {},
          webviewPermissions: true,
        },
      },`)),
    }),
    /workers\.thumbnailer\.webviewPermissions is not supported/,
  );
  await assert.rejects(
    loadCefariConfig({
      root: await projectWithConfig(baseConfig(`workers: {
        thumbnailer: {
          entry: "workers/thumbnailer.ts",
          permissions: {
            ffi: ["sqlite"],
          },
        },
      },`)),
    }),
    /workers\.thumbnailer\.permissions\.ffi is not supported/,
  );
  await assert.rejects(
    loadCefariConfig({
      root: await projectWithConfig(baseConfig(`workers: {
        thumbnailer: {
          entry: "workers/thumbnailer.ts",
          permissions: {
            read: [],
          },
        },
      },`)),
    }),
    /workers\.thumbnailer\.permissions\.read must be "none" or a non-empty string array/,
  );
  await assert.rejects(
    loadCefariConfig({
      root: await projectWithConfig(baseConfig(`workers: {
        thumbnailer: {
          entry: "workers/thumbnailer.ts",
          permissions: {
            read: ["/tmp/uploads"],
          },
        },
      },`)),
    }),
    /workers\.thumbnailer\.permissions\.read\[0\] must be a relative path or Cefari permission token/,
  );
});

test("rejects invalid deep link schemes", async () => {
  await assert.rejects(
    loadCefariConfig({
      root: await projectWithConfig(
        baseConfig(`capabilities: [{ type: "deepLinks", schemes: [] }],`),
      ),
    }),
    /capabilities\[0\]\.schemes must be a non-empty array/,
  );
  await assert.rejects(
    loadCefariConfig({
      root: await projectWithConfig(
        baseConfig(`capabilities: [deepLinks({ schemes: ["MyApp"] })],`)
          .replace(
            "import { defineConfig }",
            "import { deepLinks, defineConfig }",
          ),
      ),
    }),
    /capabilities\[0\]\.schemes\[0\] must be lowercase ASCII/,
  );
  await assert.rejects(
    loadCefariConfig({
      root: await projectWithConfig(
        baseConfig(`capabilities: [deepLinks({ schemes: ["myapp://open"] })],`)
          .replace(
            "import { defineConfig }",
            "import { deepLinks, defineConfig }",
          ),
      ),
    }),
    /capabilities\[0\]\.schemes\[0\] must be a URL scheme without :\/\//,
  );
  await assert.rejects(
    loadCefariConfig({
      root: await projectWithConfig(
        baseConfig(`capabilities: [deepLinks({ schemes: ["https"] })],`)
          .replace(
            "import { defineConfig }",
            "import { deepLinks, defineConfig }",
          ),
      ),
    }),
    /capabilities\[0\]\.schemes\[0\] must not use reserved scheme "https"/,
  );
  await assert.rejects(
    loadCefariConfig({
      root: await projectWithConfig(
        baseConfig(`capabilities: [
        deepLinks({ schemes: ["myapp"] }),
        deepLinks({ schemes: ["myapp"] }),
      ],`).replace(
          "import { defineConfig }",
          "import { deepLinks, defineConfig }",
        ),
      ),
    }),
    /capabilities\[1\]\.schemes\[0\] duplicates deep link scheme "myapp"/,
  );
});

test("rejects legacy frontend config", async () => {
  const root = await projectWithConfig(
    baseConfig(`frontend: { dist: "frontend/dist" },`),
  );

  await assert.rejects(
    loadCefariConfig({ root }),
    /frontend is no longer supported in cefari\.config\.ts; use vite instead/,
  );
});

test("reports clear validation errors", async () => {
  const root = await projectWithConfig(baseConfig(`vite: { devPort: 0 },`));

  await assert.rejects(
    loadCefariConfig({ root }),
    /vite\.devPort must be an integer from 1 to 65535/,
  );
});

test("reports clear browser validation errors", async () => {
  const root = await projectWithConfig(
    baseConfig(`browser: { webgpu: "yes" },`),
  );

  await assert.rejects(
    loadCefariConfig({ root }),
    /browser\.webgpu must be a boolean/,
  );
});

test("reports clear daemon validation errors when daemon is configured", async () => {
  await assert.rejects(
    loadCefariConfig({
      root: await projectWithConfig(
        baseConfig().replace(
          'entry: "daemon/main.ts"',
          'entry: "../daemon/main.ts"',
        ),
      ),
    }),
    /daemon\.entry must be a relative path inside the project/,
  );
});

test("rejects impossible mobile target config", async () => {
  await assert.rejects(
    loadCefariConfig({
      root: await projectWithConfig(baseConfig(`targets: {
        ios: {
          daemon: {
            entry: "daemon/main.ts",
          },
        },
      },`)),
    }),
    /targets\.ios\.daemon is not supported/,
  );
  await assert.rejects(
    loadCefariConfig({
      root: await projectWithConfig(baseConfig(`targets: {
        android: {
          capabilities: [],
        },
      },`)),
    }),
    /targets\.android\.capabilities is not supported/,
  );
});

test("creates a serializable projection", async () => {
  const root = await projectWithConfig(baseConfig(`workers: {
    thumbnailer: {
      entry: "workers/thumbnailer.ts",
      permissions: {
        read: ["$appData/uploads"],
      },
    },
  },
  vite: { root: "ui", configFile: false, devPort: 3000 },`));

  const config = await loadCefariConfig({ root });
  const serializable = toSerializableProjectConfig(config);

  assert.deepEqual(serializable.browser, {
    webgpu: false,
  });
  assert.deepEqual(serializable.vite, {
    root: "ui",
    configFile: false,
    devPort: 3000,
  });
  assert.deepEqual(serializable.workers.thumbnailer.permissions.read, [
    "$appData/uploads",
  ]);
  assert.equal(
    JSON.parse(JSON.stringify(serializable)).app.name,
    "Example App",
  );
});
