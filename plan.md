# Overitall - Project Plan

## Overview
Rust TUI combining overmind (process management) + lnav (log viewing).

See [README.md](README.md) for features and usage.
See [todo.md](todo.md) for current priorities.
See [ARCHITECTURE.md](ARCHITECTURE.md) for code structure.

## Configuration
- Config file: `.overitall.toml` (override with `--config` or `-c`)
- Procfile path + process-to-logfile mapping
- Filters auto-saved to config

## Build & Test (Rust)
```bash
cargo build              # Build
cargo test               # Run tests
cargo insta review       # Review snapshot changes
cargo run -- -c example/overitall.toml  # Run with example config
```

---

# VS Code Extension

## Structure
```
vscode-extension/
├── package.json              # Extension manifest
├── tsconfig.json
├── src/
│   ├── extension.ts          # Entry point
│   ├── ipc/
│   │   ├── client.ts         # TypeScript IPC client
│   │   └── types.ts          # Request/Response types
│   ├── providers/
│   │   ├── processTree.ts    # Sidebar tree view
│   │   └── statusBar.ts      # Status bar manager
│   ├── commands/
│   │   └── start.ts          # Start oit in terminal
│   └── utils/
│       └── socketWatcher.ts  # Watch for .oit.sock
```

## Build & Test (Extension)
```bash
cd vscode-extension
npm install
npm run compile
# Press F5 in VS Code to launch Extension Development Host
```

## IPC Protocol
Socket: `.oit.sock` in workspace root
Format: Newline-delimited JSON
```
Request:  {"command": "ping", "args": {}}
Response: {"success": true, "result": {"pong": true}}
```

Key commands: `ping`, `status`, `processes`, `summary`, `restart`, `kill`, `start`

## Code Reference

### IPC Client (TypeScript)
```typescript
import * as net from 'net';

export class OitClient {
  constructor(private socketPath: string) {}

  async send(command: string, args = {}): Promise<IpcResponse> {
    return new Promise((resolve, reject) => {
      const socket = net.createConnection(this.socketPath);
      let data = '';
      socket.on('connect', () => {
        socket.write(JSON.stringify({ command, args }) + '\n');
      });
      socket.on('data', (chunk) => {
        data += chunk.toString();
        if (data.includes('\n')) {
          socket.end();
          resolve(JSON.parse(data.trim()));
        }
      });
      socket.on('error', reject);
    });
  }

  processes() { return this.send('processes'); }
  summary() { return this.send('summary'); }
  restart(name?: string) { return this.send('restart', name ? { name } : {}); }
  kill(name: string) { return this.send('kill', { name }); }
}
```

### Start Command
```typescript
export async function startOit(workspaceFolder: vscode.WorkspaceFolder) {
  const terminal = vscode.window.createTerminal({
    name: 'Overitall',
    cwd: workspaceFolder.uri.fsPath,
  });
  terminal.show();
  terminal.sendText('oit');
  return terminal;
}
```

### Process Tree Provider
```typescript
export class ProcessTreeProvider implements vscode.TreeDataProvider<ProcessInfo> {
  private processes: ProcessInfo[] = [];

  async refresh() {
    const response = await this.client.processes();
    if (response.success) {
      this.processes = response.result;
      this._onDidChangeTreeData.fire(undefined);
    }
  }

  getTreeItem(element: ProcessInfo): vscode.TreeItem {
    const item = new vscode.TreeItem(element.name);
    item.description = element.status;
    item.iconPath = element.status === 'running'
      ? new vscode.ThemeIcon('play', new vscode.ThemeColor('testing.iconPassed'))
      : new vscode.ThemeIcon('error', new vscode.ThemeColor('testing.iconFailed'));
    return item;
  }
}
```

### Socket Watcher
```typescript
export class SocketWatcher {
  private watcher?: fs.FSWatcher;

  start() {
    this.watcher = fs.watch(this.workspacePath, (event, filename) => {
      if (filename === '.oit.sock') {
        fs.existsSync(this.socketPath) ? this.onAvailable() : this.onUnavailable();
      }
    });
  }
}
```
