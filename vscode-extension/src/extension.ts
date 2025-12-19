import * as vscode from 'vscode';
import { startOit } from './commands/start';

export function activate(context: vscode.ExtensionContext) {
  console.log('Overitall extension activated');

  context.subscriptions.push(
    vscode.commands.registerCommand('overitall.start', () => {
      startOit();
    })
  );
}

export function deactivate() {}
