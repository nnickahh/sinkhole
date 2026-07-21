export interface AdblockStats {
  totalBlocked: number;
  activeRules: number;
  activeLists: number;
  protectionEnabled: boolean;
  proxyRunning: boolean;
  proxyAddress: string;
  uptimeSeconds: number;
  bandwidthSavedBytes: number;
}

export interface AppSettings {
  protectionEnabled: boolean;
  filterListUrls: string[];
  whitelistDomains: string[];
}
