"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || (function () {
    var ownKeys = function(o) {
        ownKeys = Object.getOwnPropertyNames || function (o) {
            var ar = [];
            for (var k in o) if (Object.prototype.hasOwnProperty.call(o, k)) ar[ar.length] = k;
            return ar;
        };
        return ownKeys(o);
    };
    return function (mod) {
        if (mod && mod.__esModule) return mod;
        var result = {};
        if (mod != null) for (var k = ownKeys(mod), i = 0; i < k.length; i++) if (k[i] !== "default") __createBinding(result, mod, k[i]);
        __setModuleDefault(result, mod);
        return result;
    };
})();
Object.defineProperty(exports, "__esModule", { value: true });
exports.activate = activate;
exports.deactivate = deactivate;
const vscode = __importStar(require("vscode"));
const path = __importStar(require("path"));
const start_1 = require("./commands/start");
const client_1 = require("./ipc/client");
const processTree_1 = require("./providers/processTree");
const socketWatcher_1 = require("./utils/socketWatcher");
function activate(context) {
    console.log('Overitall extension activated');
    const workspaceFolder = vscode.workspace.workspaceFolders?.[0];
    if (!workspaceFolder) {
        return;
    }
    const workspacePath = workspaceFolder.uri.fsPath;
    const socketPath = path.join(workspacePath, '.oit.sock');
    const processTreeProvider = new processTree_1.ProcessTreeProvider();
    const socketWatcher = new socketWatcher_1.SocketWatcher(workspacePath);
    const treeView = vscode.window.createTreeView('overitallProcesses', {
        treeDataProvider: processTreeProvider,
        showCollapseAll: false,
    });
    const onSocketAvailable = () => {
        const client = new client_1.OitClient(socketPath);
        processTreeProvider.setClient(client);
    };
    const onSocketUnavailable = () => {
        processTreeProvider.setClient(undefined);
    };
    socketWatcher.start(onSocketAvailable, onSocketUnavailable);
    if (socketWatcher.isAvailable()) {
        onSocketAvailable();
    }
    context.subscriptions.push(vscode.commands.registerCommand('overitall.start', () => {
        (0, start_1.startOit)();
    }), vscode.commands.registerCommand('overitall.refresh', () => {
        processTreeProvider.refresh();
    }), vscode.commands.registerCommand('overitall.restart', async (process) => {
        if (!socketWatcher.isAvailable()) {
            vscode.window.showWarningMessage('Overitall is not running');
            return;
        }
        const client = new client_1.OitClient(socketPath);
        const response = await client.restart(process.name);
        if (response.success) {
            processTreeProvider.refresh();
        }
        else {
            vscode.window.showErrorMessage(`Failed to restart ${process.name}: ${response.error}`);
        }
    }), vscode.commands.registerCommand('overitall.stop', async (process) => {
        if (!socketWatcher.isAvailable()) {
            vscode.window.showWarningMessage('Overitall is not running');
            return;
        }
        const client = new client_1.OitClient(socketPath);
        const response = await client.kill(process.name);
        if (response.success) {
            processTreeProvider.refresh();
        }
        else {
            vscode.window.showErrorMessage(`Failed to stop ${process.name}: ${response.error}`);
        }
    }), treeView);
    context.subscriptions.push({
        dispose: () => socketWatcher.stop(),
    });
}
function deactivate() { }
//# sourceMappingURL=extension.js.map