import * as vscode from 'vscode';
import { OitClient } from '../ipc/client';

export class StatusBarManager {
  private statusBarItem: vscode.StatusBarItem;
  private client?: OitClient;
  private pollInterval?: ReturnType<typeof setInterval>;
  private static readonly POLL_INTERVAL_MS = 2000;

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
      this.startPolling();
    } else {
      this.stopPolling();
      this.setDisconnected();
    }
  }

  private startPolling(): void {
    this.stopPolling();
    this.pollInterval = setInterval(() => {
      this.refresh();
    }, StatusBarManager.POLL_INTERVAL_MS);
  }

  private stopPolling(): void {
    if (this.pollInterval) {
      clearInterval(this.pollInterval);
      this.pollInterval = undefined;
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
    this.stopPolling();
    this.statusBarItem.dispose();
  }
}
