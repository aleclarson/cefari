export interface CefariConfigInput {
  app: AppConfigInput;
  capabilities?: CapabilityInput[];
  frontend: FrontendConfigInput;
  daemon: DaemonConfigInput;
  package: PackageConfigInput;
}

export interface AppConfigInput {
  projectName: string;
  name: string;
  identifier: string;
  icon?: string;
}

export type CapabilityInput = TrayCapabilityInput;

export interface TrayCapabilityOptions {
  icon: string;
}

export interface TrayCapabilityInput extends TrayCapabilityOptions {
  type: "tray";
}

export interface FrontendConfigInput {
  dist: string;
  buildCommand?: string[];
  devCommand?: string[];
  devPort?: number;
}

export interface DaemonConfigInput {
  entry: string;
}

export interface PackageConfigInput {
  productName: string;
  version: string;
}

export declare function defineConfig(config: CefariConfigInput): CefariConfigInput;
export declare function tray(config: TrayCapabilityOptions): TrayCapabilityInput;
