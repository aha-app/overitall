import * as vscode from 'vscode';
import * as path from 'path';
import { startOit } from './commands/start';
import { OitClient } from './ipc/client';
import { ProcessTreeProvider } from './providers/processTree';
import { SocketWatcher } from './utils/socketWatcher';
import { ProcessInfo } from './ipc/types';

export function activate(context: vscode.ExtensionContext) {
  console.log('Overitall extension activated');

  const workspaceFolder = vscode.workspace.workspaceFolders?.[0];
  if (!workspaceFolder) {
    return;
  }

  const workspacePath = workspaceFolder.uri.fsPath;
  const socketPath = path.join(workspacePath, '.oit.sock');

  const processTreeProvider = new ProcessTreeProvider();
  const socketWatcher = new SocketWatcher(workspacePath);

  const treeView = vscode.window.createTreeView('overitallProcesses', {
    treeDataProvider: processTreeProvider,
    showCollapseAll: false,
  });

  const onSocketAvailable = () => {
    const client = new OitClient(socketPath);
    processTreeProvider.setClient(client);
  };

  const onSocketUnavailable = () => {
    processTreeProvider.setClient(undefined);
  };

  socketWatcher.start(onSocketAvailable, onSocketUnavailable);

  if (socketWatcher.isAvailable()) {
    onSocketAvailable();
  }

  context.subscriptions.push(
    vscode.commands.registerCommand('overitall.start', () => {
      startOit();
    }),

    vscode.commands.registerCommand('overitall.refresh', () => {
      processTreeProvider.refresh();
    }),

    vscode.commands.registerCommand('overitall.restart', async (process: ProcessInfo) => {
      if (!socketWatcher.isAvailable()) {
        vscode.window.showWarningMessage('Overitall is not running');
        return;
      }
      const client = new OitClient(socketPath);
      const response = await client.restart(process.name);
      if (response.success) {
        processTreeProvider.refresh();
      } else {
        vscode.window.showErrorMessage(`Failed to restart ${process.name}: ${response.error}`);
      }
    }),

    vscode.commands.registerCommand('overitall.stop', async (process: ProcessInfo) => {
      if (!socketWatcher.isAvailable()) {
        vscode.window.showWarningMessage('Overitall is not running');
        return;
      }
      const client = new OitClient(socketPath);
      const response = await client.kill(process.name);
      if (response.success) {
        processTreeProvider.refresh();
      } else {
        vscode.window.showErrorMessage(`Failed to stop ${process.name}: ${response.error}`);
      }
    }),

    treeView,
  );

  context.subscriptions.push({
    dispose: () => socketWatcher.stop(),
  });
}

export function deactivate() {}
