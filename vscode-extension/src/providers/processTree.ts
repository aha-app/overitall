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
    item.description = element.status;
    item.iconPath = this.getIconForStatus(element.status);
    item.contextValue = 'process';
    if (element.error) {
      item.tooltip = element.error;
    }
    return item;
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
