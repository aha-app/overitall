import * as vscode from 'vscode';
import * as path from 'path';
import { startOit, setExtensionContext, setSocketChecker, showOitTerminal, getOitTerminal, clearOitTerminal } from './commands/start';
import { OitClient } from './ipc/client';
import { ProcessTreeProvider } from './providers/processTree';
import { StatusBarManager } from './providers/statusBar';
import { SocketWatcher } from './utils/socketWatcher';
import { ProcessInfo } from './ipc/types';
import { log, getOutputChannel, disposeLogger } from './utils/logger';

export function activate(context: vscode.ExtensionContext) {
  log('Overitall extension activating...');

  // Set context for development mode detection
  setExtensionContext(context);

  const workspaceFolder = vscode.workspace.workspaceFolders?.[0];
  if (!workspaceFolder) {
    log('No workspace folder found, extension will not activate');
    return;
  }

  const workspacePath = workspaceFolder.uri.fsPath;
  const socketPath = path.join(workspacePath, '.oit.sock');
  log(`Workspace: ${workspacePath}`);
  log(`Socket path: ${socketPath}`);

  const processTreeProvider = new ProcessTreeProvider();
  const statusBarManager = new StatusBarManager();
  const socketWatcher = new SocketWatcher(workspacePath);

  // Set up socket checker for start command to detect if oit is running
  setSocketChecker(() => socketWatcher.isAvailable());

  statusBarManager.show();

  const treeView = vscode.window.createTreeView('overitallProcesses', {
    treeDataProvider: processTreeProvider,
    showCollapseAll: false,
  });

  const onSocketAvailable = () => {
    log('Socket available - connecting to oit');
    const client = new OitClient(socketPath);
    processTreeProvider.setClient(client);
    statusBarManager.setClient(client);
  };

  const onSocketUnavailable = () => {
    log('Socket unavailable - disconnecting from oit');
    processTreeProvider.setClient(undefined);
    statusBarManager.setClient(undefined);
  };

  socketWatcher.start(onSocketAvailable, onSocketUnavailable);

  if (socketWatcher.isAvailable()) {
    log('Socket already exists on activation');
    onSocketAvailable();
  }

  context.subscriptions.push(getOutputChannel());

  // Handle terminal close events to clean up our terminal reference
  context.subscriptions.push(
    vscode.window.onDidCloseTerminal((terminal) => {
      const oitTerminal = getOitTerminal();
      if (oitTerminal && terminal === oitTerminal) {
        log('Overitall terminal was closed');
        clearOitTerminal();
      }
    })
  );

  context.subscriptions.push(
    vscode.commands.registerCommand('overitall.start', () => {
      log('Command: overitall.start');
      startOit();
    }),

    vscode.commands.registerCommand('overitall.showTerminal', () => {
      log('Command: overitall.showTerminal');
      showOitTerminal();
    }),

    vscode.commands.registerCommand('overitall.refresh', () => {
      processTreeProvider.refresh();
      statusBarManager.refresh();
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
        statusBarManager.refresh();
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
        statusBarManager.refresh();
      } else {
        vscode.window.showErrorMessage(`Failed to stop ${process.name}: ${response.error}`);
      }
    }),

    treeView,
    statusBarManager,
    { dispose: () => processTreeProvider.dispose() },
  );

  context.subscriptions.push({
    dispose: () => socketWatcher.stop(),
  });
}

export function deactivate() {}
