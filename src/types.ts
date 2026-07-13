export interface Remote {
  name: string;
  type: string;
}

export interface TransferRecord {
  id: string;
  remote_src: string;
  remote_dest: string;
  status: string;
  progress: number;
  speed: string | null;
  started_at: string;
  completed_at: string | null;
  error_message: string | null;
}

export interface MountRecord {
  id: string;
  remote: string;
  mount_point: string;
  status: string;
}

export interface ProgressPayload {
  process_id: string;
  transferred: string;
  total: string;
  percent: number;
  speed: string;
  eta: string;
}

export interface MountInfo {
  id: string;
  remote: string;
  mount_point: string;
  status: string;
  pid: number | null;
}

export interface Task {
  id: string;
  name: string;
  slug: string;
  source_provider: string;
  source_config: Record<string, unknown>;
  dest_provider: string;
  dest_config: Record<string, unknown>;
  operation: "copy" | "sync" | "move" | "bisync";
  exclude_patterns: string[];
  cron_expr: string;
  enabled: boolean;
  created_at: string;
  updated_at: string;
}

export interface ProviderOption {
  Name: string;
  Help: string;
  Default: unknown;
  Required: boolean;
  Type: string;
  Advanced: boolean;
  IsPassword: boolean;
  Exclusive: boolean;
  Examples: Array<{ Value: string; Help: string }> | null;
}

export interface Provider {
  Name: string;
  Description: string;
  Prefix: string;
  Options: ProviderOption[];
}

export function generateSlug(name: string): string {
  return name
    .toLowerCase()
    .replace(/[şŞ]/g, 's')
    .replace(/[ıIİ]/g, 'i')
    .replace(/[üÜ]/g, 'u')
    .replace(/[öÖ]/g, 'o')
    .replace(/[çÇ]/g, 'c')
    .replace(/[ğĞ]/g, 'g')
    .replace(/[\s_]+/g, '-')
    .replace(/[^a-z0-9-]/g, '-')
    .replace(/-+/g, '-')
    .replace(/^-+|-+$/g, '');
}
