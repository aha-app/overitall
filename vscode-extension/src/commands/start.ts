import * as vscode from 'vscode';
import * as fs from 'fs';
import * as path from 'path';
import { log } from '../utils/logger';

let extensionContext: vscode.ExtensionContext | undefined;

export function setExtensionContext(context: vscode.ExtensionContext): void {
  extensionContext = context;
}

export async function startOit(): Promise<vscode.Terminal | undefined> {
  const workspaceFolder = vscode.workspace.workspaceFolders?.[0];
  if (!workspaceFolder) {
    vscode.window.showErrorMessage('No workspace folder open');
    return undefined;
  }

  const oitCommand = getOitCommand(workspaceFolder.uri.fsPath);

  const terminal = vscode.window.createTerminal({
    name: 'Overitall',
    cwd: workspaceFolder.uri.fsPath,
  });
  terminal.show();
  terminal.sendText(oitCommand);
  return terminal;
}

function getOitCommand(workspacePath: string): string {
  // In development mode, prefer local binary from target directory
  if (extensionContext?.extensionMode === vscode.ExtensionMode.Development) {
    log('Development mode detected, looking for local oit binary');
    // Check for release binary first, then debug
    const releaseBinary = path.join(workspacePath, 'target', 'release', 'oit');
    const debugBinary = path.join(workspacePath, 'target', 'debug', 'oit');

    if (fs.existsSync(releaseBinary)) {
      log(`Using release binary: ${releaseBinary}`);
      return releaseBinary;
    }
    if (fs.existsSync(debugBinary)) {
      log(`Using debug binary: ${debugBinary}`);
      return debugBinary;
    }
    log('No local binary found, falling back to system oit');
  }

  // Fall back to system oit
  log('Using system oit');
  return 'oit';
}
