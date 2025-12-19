import * as vscode from 'vscode';

export function activate(context: vscode.ExtensionContext) {
  console.log('Overitall extension activated');

  context.subscriptions.push(
    vscode.commands.registerCommand('overitall.start', () => {
      vscode.window.showInformationMessage('Starting Overitall...');
    })
  );
}

export function deactivate() {}
