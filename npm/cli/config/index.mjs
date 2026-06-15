export function defineConfig(config) {
  return config;
}

export function tray(config) {
  return { type: "tray", ...config };
}
