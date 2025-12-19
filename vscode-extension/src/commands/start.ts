import * as vscode from 'vscode';

export async function startOit(): Promise<vscode.Terminal | undefined> {
  const workspaceFolder = vscode.workspace.workspaceFolders?.[0];
  if (!workspaceFolder) {
    vscode.window.showErrorMessage('No workspace folder open');
    return undefined;
  }

  const terminal = vscode.window.createTerminal({
    name: 'Overitall',
    cwd: workspaceFolder.uri.fsPath,
  });
  terminal.show();
  terminal.sendText('oit');
  return terminal;
}
