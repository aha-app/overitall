import * as vscode from 'vscode';
import { OitClient } from '../ipc/client';
import { ProcessInfo } from '../ipc/types';

export class ProcessTreeProvider implements vscode.TreeDataProvider<ProcessInfo> {
  private _onDidChangeTreeData = new vscode.EventEmitter<ProcessInfo | undefined>();
  readonly onDidChangeTreeData = this._onDidChangeTreeData.event;

  private processes: ProcessInfo[] = [];
  private client?: OitClient;

  setClient(client: OitClient | undefined): void {
    this.client = client;
    if (client) {
      this.refresh();
    } else {
      this.processes = [];
      this._onDidChangeTreeData.fire(undefined);
    }
  }

  async refresh(): Promise<void> {
    if (!this.client) {
      return;
    }
    try {
      const response = await this.client.processes();
      if (response) {
        this.processes = response.processes;
        this._onDidChangeTreeData.fire(undefined);
      }
    } catch {
      this.processes = [];
      this._onDidChangeTreeData.fire(undefined);
    }
  }

  getTreeItem(element: ProcessInfo): vscode.TreeItem {
    const item = new vscode.TreeItem(element.name);
    // Use custom label if available, otherwise fall back to status
    item.description = element.custom_label || element.status;
    item.iconPath = this.getIconForProcess(element);
    item.contextValue = 'process';
    if (element.error) {
      item.tooltip = element.error;
    }
    return item;
  }

  private getIconForProcess(process: ProcessInfo): vscode.ThemeIcon {
    // If custom color is set, use it with a play icon (for running processes)
    if (process.custom_color && process.status === 'running') {
      const themeColor = this.getThemeColorForCustomColor(process.custom_color);
      return new vscode.ThemeIcon('play', themeColor);
    }
    // Fall back to status-based icons
    return this.getIconForStatus(process.status);
  }

  private getThemeColorForCustomColor(color: string): vscode.ThemeColor | undefined {
    switch (color) {
      case 'green':
        return new vscode.ThemeColor('testing.iconPassed');
      case 'yellow':
        return new vscode.ThemeColor('editorWarning.foreground');
      case 'red':
        return new vscode.ThemeColor('testing.iconFailed');
      case 'blue':
        return new vscode.ThemeColor('textLink.foreground');
      case 'cyan':
        return new vscode.ThemeColor('terminal.ansiCyan');
      case 'magenta':
        return new vscode.ThemeColor('terminal.ansiMagenta');
      default:
        return undefined;
    }
  }

  private getIconForStatus(status: string): vscode.ThemeIcon {
    switch (status) {
      case 'running':
        return new vscode.ThemeIcon('play', new vscode.ThemeColor('testing.iconPassed'));
      case 'stopped':
        return new vscode.ThemeIcon('debug-stop', new vscode.ThemeColor('disabledForeground'));
      case 'failed':
        return new vscode.ThemeIcon('error', new vscode.ThemeColor('testing.iconFailed'));
      default:
        return new vscode.ThemeIcon('circle-outline');
    }
  }

  getChildren(element?: ProcessInfo): ProcessInfo[] {
    if (element) {
      return [];
    }
    return this.processes;
  }

  getParent(): ProcessInfo | undefined {
    return undefined;
  }
}
