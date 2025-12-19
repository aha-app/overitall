import * as vscode from 'vscode';

let outputChannel: vscode.OutputChannel | undefined;

export function getOutputChannel(): vscode.OutputChannel {
  if (!outputChannel) {
    outputChannel = vscode.window.createOutputChannel('Overitall');
  }
  return outputChannel;
}

export function log(message: string): void {
  const timestamp = new Date().toISOString().slice(11, 23);
  getOutputChannel().appendLine(`[${timestamp}] ${message}`);
}

export function disposeLogger(): void {
  outputChannel?.dispose();
  outputChannel = undefined;
}
