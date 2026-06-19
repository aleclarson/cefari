# `browser`

`browser` config controls opt-in behavior for the embedded CEF browser.

```ts
import { defineConfig } from "cefari";

export default defineConfig({
  app: {
    projectName: "my-cefari-app",
    name: "My Cefari App",
    identifier: "dev.cefari.my-cefari-app",
  },
  browser: {
    webgpu: true,
  },
  package: {
    productName: "My Cefari App",
    version: "0.1.0",
  },
});
```

| Field | Required | Description |
| --- | --- | --- |
| `webgpu` | No | Enables Chromium WebGPU support for the embedded browser. Defaults to `false`. |

## `webgpu`

Set `webgpu: true` when the app frontend uses the browser WebGPU API.

Cefari enables the Chromium switches needed by CEF before browser startup. On
Linux, Cefari also enables Chromium's Vulkan feature because WebGPU support
depends on the Vulkan backend there.

WebGPU remains subject to Chromium, operating system, GPU, and driver support.
When WebGPU is unavailable, frontend code should handle `navigator.gpu` or
`requestAdapter()` being absent or returning no adapter.
