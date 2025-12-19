import * as vscode from 'vscode';
import { OitClient } from '../ipc/client';

export class StatusBarManager {
  private statusBarItem: vscode.StatusBarItem;
  private client?: OitClient;

  constructor() {
    this.statusBarItem = vscode.window.createStatusBarItem(
      vscode.StatusBarAlignment.Left,
      100
    );
    this.statusBarItem.command = 'overitall.refresh';
    this.setDisconnected();
  }

  setClient(client: OitClient | undefined): void {
    this.client = client;
    if (client) {
      this.refresh();
    } else {
      this.setDisconnected();
    }
  }

  async refresh(): Promise<void> {
    if (!this.client) {
      return;
    }
    try {
      const summary = await this.client.summary();
      if (summary) {
        const { running, total } = summary.processes;
        this.statusBarItem.text = `$(server-process) oit: ${running}/${total}`;
        this.statusBarItem.tooltip = `Overitall: ${running} of ${total} processes running`;
        if (running === total) {
          this.statusBarItem.backgroundColor = undefined;
        } else {
          this.statusBarItem.backgroundColor = new vscode.ThemeColor(
            'statusBarItem.warningBackground'
          );
        }
      }
    } catch {
      this.setDisconnected();
    }
  }

  private setDisconnected(): void {
    this.statusBarItem.text = '$(server-process) oit: --';
    this.statusBarItem.tooltip = 'Overitall: not running';
    this.statusBarItem.backgroundColor = undefined;
  }

  show(): void {
    this.statusBarItem.show();
  }

  dispose(): void {
    this.statusBarItem.dispose();
  }
}
