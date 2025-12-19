import * as fs from 'fs';
import * as path from 'path';
import { log } from './logger';

export class SocketWatcher {
  private watcher?: fs.FSWatcher;
  private pollInterval?: NodeJS.Timeout;
  private socketPath: string;
  private wasAvailable: boolean = false;
  private onAvailableCallback?: () => void;
  private onUnavailableCallback?: () => void;

  constructor(private workspacePath: string) {
    this.socketPath = path.join(workspacePath, '.oit.sock');
    log(`SocketWatcher: watching for socket at ${this.socketPath}`);
  }

  start(onAvailable: () => void, onUnavailable: () => void): void {
    this.onAvailableCallback = onAvailable;
    this.onUnavailableCallback = onUnavailable;
    this.wasAvailable = fs.existsSync(this.socketPath);
    log(`SocketWatcher: initial socket state: ${this.wasAvailable ? 'available' : 'not available'}`);

    try {
      this.watcher = fs.watch(this.workspacePath, (event, filename) => {
        if (filename === '.oit.sock') {
          log(`SocketWatcher: fs.watch detected change: ${event} ${filename}`);
          this.checkAndNotify();
        }
      });
      log(`SocketWatcher: fs.watch started on ${this.workspacePath}`);

      this.watcher.on('error', (err) => {
        log(`SocketWatcher: fs.watch error: ${err.message}`);
        this.watcher?.close();
        this.watcher = undefined;
      });
    } catch (err) {
      log(`SocketWatcher: failed to start fs.watch: ${err}`);
    }

    // Poll every 2 seconds as fallback (fs.watch can be unreliable on macOS for sockets)
    this.pollInterval = setInterval(() => {
      this.checkAndNotify();
    }, 2000);
    log('SocketWatcher: polling started (2s interval)');
  }

  private checkAndNotify(): void {
    const isNowAvailable = fs.existsSync(this.socketPath);
    if (isNowAvailable !== this.wasAvailable) {
      log(`SocketWatcher: socket state changed: ${this.wasAvailable} -> ${isNowAvailable}`);
      this.wasAvailable = isNowAvailable;
      if (isNowAvailable) {
        log('SocketWatcher: calling onAvailable callback');
        this.onAvailableCallback?.();
      } else {
        log('SocketWatcher: calling onUnavailable callback');
        this.onUnavailableCallback?.();
      }
    }
  }

  stop(): void {
    log('SocketWatcher: stopping');
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
