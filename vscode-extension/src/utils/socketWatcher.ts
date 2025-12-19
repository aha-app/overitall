import * as fs from 'fs';
import * as path from 'path';

export class SocketWatcher {
  private watcher?: fs.FSWatcher;
  private socketPath: string;

  constructor(private workspacePath: string) {
    this.socketPath = path.join(workspacePath, '.oit.sock');
  }

  start(onAvailable: () => void, onUnavailable: () => void): void {
    try {
      this.watcher = fs.watch(this.workspacePath, (event, filename) => {
        if (filename === '.oit.sock') {
          if (fs.existsSync(this.socketPath)) {
            onAvailable();
          } else {
            onUnavailable();
          }
        }
      });

      this.watcher.on('error', () => {
        // Directory might not exist or be inaccessible
        this.stop();
      });
    } catch {
      // Workspace directory might not exist
    }
  }

  stop(): void {
    if (this.watcher) {
      this.watcher.close();
      this.watcher = undefined;
    }
  }

  isAvailable(): boolean {
    return fs.existsSync(this.socketPath);
  }
}
