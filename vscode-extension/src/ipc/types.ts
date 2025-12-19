export interface IpcRequest {
  command: string;
  args?: Record<string, unknown>;
}

export interface IpcResponse {
  success: boolean;
  result?: unknown;
  error?: string;
}

export interface ProcessInfo {
  name: string;
  status: string;
  error?: string;
  custom_label?: string;
  custom_color?: string;
}

export interface ProcessesResponse {
  processes: ProcessInfo[];
}

export interface SummaryResponse {
  status: {
    version: string;
    running: boolean;
  };
  processes: {
    total: number;
    running: number;
    failed: number;
    stopped: number;
    details: ProcessInfo[];
  };
  logs: {
    total_lines: number;
    visible_lines: number;
  };
  errors: {
    recent_count: number;
  };
}
