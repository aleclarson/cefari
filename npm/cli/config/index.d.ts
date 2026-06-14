export interface CefariConfigInput {
  app: AppConfigInput;
  frontend: FrontendConfigInput;
  daemon: DaemonConfigInput;
  package: PackageConfigInput;
}

export interface AppConfigInput {
  projectName: string;
  name: string;
  identifier: string;
  icon?: string;
  trayIcon?: string;
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
