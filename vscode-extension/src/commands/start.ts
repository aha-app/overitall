import * as vscode from 'vscode';
import * as fs from 'fs';
import * as path from 'path';
import { log } from '../utils/logger';

let extensionContext: vscode.ExtensionContext | undefined;
let oitTerminal: vscode.Terminal | undefined;
let socketChecker: (() => boolean) | undefined;

export function setExtensionContext(context: vscode.ExtensionContext): void {
  extensionContext = context;
}

export function setSocketChecker(checker: () => boolean): void {
  socketChecker = checker;
}

export function getOitTerminal(): vscode.Terminal | undefined {
  return oitTerminal;
}

export function clearOitTerminal(): void {
  oitTerminal = undefined;
}

export async function startOit(): Promise<vscode.Terminal | undefined> {
  // If terminal already exists and is still valid, check if oit is running
  if (oitTerminal) {
    const terminals = vscode.window.terminals;
    if (terminals.includes(oitTerminal)) {
      const isOitRunning = socketChecker?.() ?? false;
      if (isOitRunning) {
        log('Reusing existing Overitall terminal (oit is running)');
        oitTerminal.show();
        return oitTerminal;
      }
      // Terminal exists but oit is not running - restart it in the same terminal
      log('Terminal exists but oit not running, restarting oit');
      oitTerminal.show();
      const workspaceFolder = vscode.workspace.workspaceFolders?.[0];
      if (workspaceFolder) {
        const oitCommand = getOitCommand(workspaceFolder.uri.fsPath);
        oitTerminal.sendText(oitCommand);
      }
      return oitTerminal;
    }
    // Terminal was closed, clear reference
    oitTerminal = undefined;
  }
  const workspaceFolder = vscode.workspace.workspaceFolders?.[0];
  if (!workspaceFolder) {
    vscode.window.showErrorMessage('No workspace folder open');
    return undefined;
  }

  const oitCommand = getOitCommand(workspaceFolder.uri.fsPath);

  oitTerminal = vscode.window.createTerminal({
    name: 'Overitall',
    cwd: workspaceFolder.uri.fsPath,
  });
  oitTerminal.show();
  oitTerminal.sendText(oitCommand);
  return oitTerminal;
}

export function showOitTerminal(): void {
  log(`showOitTerminal called, oitTerminal=${oitTerminal ? 'set' : 'undefined'}`);
  if (oitTerminal) {
    const terminals = vscode.window.terminals;
    const found = terminals.some(t => t === oitTerminal);
    log(`Terminal count: ${terminals.length}, found in list: ${found}`);
    if (found) {
      log('Showing terminal');
      oitTerminal.show(false); // false = take focus
      return;
    }
    log('Terminal not found in list, clearing reference');
    oitTerminal = undefined;
  }
  vscode.window.showInformationMessage('Overitall terminal is not running. Use "Start Overitall" to launch it.');
}

function getOitCommand(workspacePath: string): string {
  // In development mode, prefer local binary from target directory
  if (extensionContext?.extensionMode === vscode.ExtensionMode.Development) {
    log('Development mode detected, looking for local oit binary');

    // Extension is in vscode-extension/, so go up one level to find target/
    const extensionPath = extensionContext.extensionPath;
    const repoRoot = path.dirname(extensionPath);
    log(`Extension path: ${extensionPath}`);
    log(`Repo root: ${repoRoot}`);

    // Check for release binary first, then debug
    const releaseBinary = path.join(repoRoot, 'target', 'release', 'oit');
    const debugBinary = path.join(repoRoot, 'target', 'debug', 'oit');

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
