export interface AdblockStats {
  totalBlocked: number;
  blockedAds: number;
  blockedTrackers: number;
  blockedTelemetry: number;
  blockedCustom: number;
  activeRules: number;
  activeAdRules: number;
  activeTrackerRules: number;
  activeTelemetryRules: number;
  activeCustomRules: number;
  activeLists: number;
  protectionEnabled: boolean;
  proxyRunning: boolean;
  proxyAddress: string;
  requestsProcessed: number;
  lastRequestAgeSeconds: number | null;
  uptimeSeconds: number;
  bandwidthSavedBytes: number;
}

export interface AppSettings {
  protectionEnabled: boolean;
  systemProxyEnabled: boolean;
  filterListUrls: string[];
  whitelistDomains: string[];
}

export type RuleCategory = "ad" | "tracker" | "telemetry" | "custom";

export interface LinkInspection {
  blocked: boolean;
  category: RuleCategory | null;
}
