export type FeatureCategory = "Bridge" | "Desktop" | "System" | "Files" | "Workers" | "Diagnostics";

export type FeatureContext = {
  log(message: string, data?: unknown): void;
};

export type FeatureModule = {
  id: string;
  name: string;
  category: FeatureCategory;
  description: string;
  run(context: FeatureContext): Promise<unknown>;
};
