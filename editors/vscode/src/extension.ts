import * as vscode from 'vscode';
import { LspBridgeProvider } from './lspBridgeProvider';
import { DiagnosticExporter } from './diagnosticExporter';
import { HistoryView } from './historyView';
import { QuickFixProvider } from './quickFixProvider';

let diagnosticWatcher: vscode.Disposable | undefined;

export function activate(context: vscode.ExtensionContext) {
    console.log('LSP Bridge extension activated');

    const lspBridge = new LspBridgeProvider(context);
    const exporter = new DiagnosticExporter(lspBridge);
    const historyView = new HistoryView(lspBridge);
    const quickFixProvider = new QuickFixProvider(lspBridge);

    // Register commands
    context.subscriptions.push(
        vscode.commands.registerCommand('lsp-bridge.exportDiagnostics', async () => {
            await exportDiagnosticsCommand(exporter);
        })
    );

    context.subscriptions.push(
        vscode.commands.registerCommand('lsp-bridge.exportToClipboard', async () => {
            await exportToClipboardCommand(exporter);
        })
    );

    context.subscriptions.push(
        vscode.commands.registerCommand('lsp-bridge.watchDiagnostics', async () => {
            await startWatchingCommand(exporter);
        })
    );

    context.subscriptions.push(
        vscode.commands.registerCommand('lsp-bridge.stopWatching', () => {
            stopWatchingCommand();
        })
    );

    context.subscriptions.push(
        vscode.commands.registerCommand('lsp-bridge.showHistory', async () => {
            await historyView.show();
        })
    );

    context.subscriptions.push(
        vscode.commands.registerCommand('lsp-bridge.applyQuickFixes', async () => {
            await quickFixProvider.applyFixes();
        })
    );

    // Register event listeners
    if (vscode.workspace.getConfiguration('lsp-bridge').get('autoExportOnSave')) {
        context.subscriptions.push(
            vscode.workspace.onDidSaveTextDocument(async (document) => {
                await exporter.exportForDocument(document);
            })
        );
    }

    // Create status bar item
    const statusBarItem = vscode.window.createStatusBarItem(
        vscode.StatusBarAlignment.Right,
        100
    );
    statusBarItem.text = '$(warning) LSP Bridge';
    statusBarItem.tooltip = 'Click to export diagnostics';
    statusBarItem.command = 'lsp-bridge.exportDiagnostics';
    context.subscriptions.push(statusBarItem);

    // Update status bar with diagnostic count
    context.subscriptions.push(
        vscode.languages.onDidChangeDiagnostics((e) => {
            updateStatusBar(statusBarItem);
        })
    );

    updateStatusBar(statusBarItem);
    statusBarItem.show();
}

export function deactivate() {
    if (diagnosticWatcher) {
        diagnosticWatcher.dispose();
    }
}

async function exportDiagnosticsCommand(exporter: DiagnosticExporter) {
    try {
        const options = await getExportOptions();
        if (!options) {
            return;
        }

        const result = await exporter.exportDiagnostics(options);
        
        if (result.outputPath) {
            const openFile = await vscode.window.showInformationMessage(
                `Diagnostics exported to ${result.outputPath}`,
                'Open File'
            );
            
            if (openFile) {
                const doc = await vscode.workspace.openTextDocument(result.outputPath);
                await vscode.window.showTextDocument(doc);
            }
        } else {
            vscode.window.showInformationMessage('Diagnostics exported successfully');
        }
    } catch (error) {
        vscode.window.showErrorMessage(`Failed to export diagnostics: ${error}`);
    }
}

async function exportToClipboardCommand(exporter: DiagnosticExporter) {
    try {
        const result = await exporter.exportToClipboard();
        vscode.window.showInformationMessage(
            `Exported ${result.diagnosticCount} diagnostics to clipboard`
        );
    } catch (error) {
        vscode.window.showErrorMessage(`Failed to export to clipboard: ${error}`);
    }
}

async function startWatchingCommand(exporter: DiagnosticExporter) {
    if (diagnosticWatcher) {
        vscode.window.showWarningMessage('Already watching diagnostics');
        return;
    }

    try {
        diagnosticWatcher = await exporter.startWatching();
        vscode.window.showInformationMessage('Started watching diagnostics');
    } catch (error) {
        vscode.window.showErrorMessage(`Failed to start watching: ${error}`);
    }
}

function stopWatchingCommand() {
    if (!diagnosticWatcher) {
        vscode.window.showWarningMessage('Not currently watching diagnostics');
        return;
    }

    diagnosticWatcher.dispose();
    diagnosticWatcher = undefined;
    vscode.window.showInformationMessage('Stopped watching diagnostics');
}

async function getExportOptions(): Promise<ExportOptions | undefined> {
    const config = vscode.workspace.getConfiguration('lsp-bridge');
    
    // Ask for export location
    const exportLocation = await vscode.window.showQuickPick(
        ['File', 'Clipboard', 'Both'],
        { placeHolder: 'Where to export diagnostics?' }
    );
    
    if (!exportLocation) {
        return undefined;
    }

    let outputPath: string | undefined;
    if (exportLocation === 'File' || exportLocation === 'Both') {
        const defaultPath = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath || '';
        const uri = await vscode.window.showSaveDialog({
            defaultUri: vscode.Uri.file(`${defaultPath}/diagnostics.md`),
            filters: {
                'Markdown': ['md'],
                'JSON': ['json'],
                'All': ['*']
            }
        });
        
        if (!uri) {
            return undefined;
        }
        
        outputPath = uri.fsPath;
    }

    // Ask for scope
    const scope = await vscode.window.showQuickPick(
        ['Workspace', 'Current File', 'Open Files', 'Errors Only'],
        { placeHolder: 'Select diagnostic scope' }
    );
    
    if (!scope) {
        return undefined;
    }

    return {
        format: config.get('exportFormat') as string,
        privacyLevel: config.get('privacyLevel') as string,
        includeContext: config.get('includeContext') as boolean,
        contextLines: config.get('contextLines') as number,
        outputPath,
        toClipboard: exportLocation === 'Clipboard' || exportLocation === 'Both',
        scope
    };
}

function updateStatusBar(statusBarItem: vscode.StatusBarItem) {
    const diagnostics = vscode.languages.getDiagnostics();
    let errorCount = 0;
    let warningCount = 0;

    for (const [uri, fileDiagnostics] of diagnostics) {
        for (const diagnostic of fileDiagnostics) {
            if (diagnostic.severity === vscode.DiagnosticSeverity.Error) {
                errorCount++;
            } else if (diagnostic.severity === vscode.DiagnosticSeverity.Warning) {
                warningCount++;
            }
        }
    }

    if (errorCount > 0) {
        statusBarItem.text = `$(error) ${errorCount} $(warning) ${warningCount} LSP Bridge`;
        statusBarItem.backgroundColor = new vscode.ThemeColor('statusBarItem.errorBackground');
    } else if (warningCount > 0) {
        statusBarItem.text = `$(warning) ${warningCount} LSP Bridge`;
        statusBarItem.backgroundColor = new vscode.ThemeColor('statusBarItem.warningBackground');
    } else {
        statusBarItem.text = '$(check) LSP Bridge';
        statusBarItem.backgroundColor = undefined;
    }
}

interface ExportOptions {
    format: string;
    privacyLevel: string;
    includeContext: boolean;
    contextLines: number;
    outputPath?: string;
    toClipboard: boolean;
    scope: string;
}