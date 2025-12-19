import * as fs from 'fs';
import * as path from 'path';

export class SocketWatcher {
  private watcher?: fs.FSWatcher;
  private pollInterval?: NodeJS.Timeout;
  private socketPath: string;
  private wasAvailable: boolean = false;
  private onAvailableCallback?: () => void;
  private onUnavailableCallback?: () => void;

  constructor(private workspacePath: string) {
    this.socketPath = path.join(workspacePath, '.oit.sock');
  }

  start(onAvailable: () => void, onUnavailable: () => void): void {
    this.onAvailableCallback = onAvailable;
    this.onUnavailableCallback = onUnavailable;
    this.wasAvailable = fs.existsSync(this.socketPath);

    try {
      this.watcher = fs.watch(this.workspacePath, (event, filename) => {
        if (filename === '.oit.sock') {
          this.checkAndNotify();
        }
      });

      this.watcher.on('error', () => {
        this.watcher?.close();
        this.watcher = undefined;
      });
    } catch {
      // Workspace directory might not exist
    }

    // Poll every 2 seconds as fallback (fs.watch can be unreliable on macOS for sockets)
    this.pollInterval = setInterval(() => {
      this.checkAndNotify();
    }, 2000);
  }

  private checkAndNotify(): void {
    const isNowAvailable = fs.existsSync(this.socketPath);
    if (isNowAvailable !== this.wasAvailable) {
      this.wasAvailable = isNowAvailable;
      if (isNowAvailable) {
        this.onAvailableCallback?.();
      } else {
        this.onUnavailableCallback?.();
      }
    }
  }

  stop(): void {
    if (this.watcher) {
      this.watcher.close();
      this.watcher = undefined;
    }
    if (this.pollInterval) {
      clearInterval(this.pollInterval);
      this.pollInterval = undefined;
    }
  }

  isAvailable(): boolean {
    return fs.existsSync(this.socketPath);
  }
}
