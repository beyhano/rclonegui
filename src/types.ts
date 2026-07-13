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
