import * as vscode from 'vscode';
import { LspBridgeProvider } from './lspBridgeProvider';
import * as fs from 'fs/promises';

export class DiagnosticExporter {
    constructor(private lspBridge: LspBridgeProvider) {}

    async exportDiagnostics(options: {
        format: string;
        privacyLevel: string;
        includeContext: boolean;
        contextLines: number;
        outputPath?: string;
        toClipboard: boolean;
        scope: string;
    }): Promise<{ outputPath?: string; diagnosticCount: number }> {
        const files = await this.getFilesForScope(options.scope);
        const errorsOnly = options.scope === 'Errors Only';

        const result = await this.lspBridge.exportDiagnostics({
            format: options.format,
            privacyLevel: options.privacyLevel,
            includeContext: options.includeContext,
            contextLines: options.contextLines,
            outputPath: options.outputPath,
            errorsOnly,
            files
        });

        if (options.toClipboard) {
            await vscode.env.clipboard.writeText(result);
        }

        // Count diagnostics in result
        const diagnosticCount = this.countDiagnostics(result);

        return {
            outputPath: options.outputPath,
            diagnosticCount
        };
    }

    async exportToClipboard(): Promise<{ diagnosticCount: number }> {
        const config = vscode.workspace.getConfiguration('lsp-bridge');
        
        const result = await this.lspBridge.exportDiagnostics({
            format: config.get('exportFormat') as string,
            privacyLevel: config.get('privacyLevel') as string,
            includeContext: config.get('includeContext') as boolean,
            contextLines: config.get('contextLines') as number
        });

        await vscode.env.clipboard.writeText(result);
        
        return {
            diagnosticCount: this.countDiagnostics(result)
        };
    }

    async exportForDocument(document: vscode.TextDocument): Promise<void> {
        const config = vscode.workspace.getConfiguration('lsp-bridge');
        
        // Only export if there are diagnostics for this file
        const diagnostics = vscode.languages.getDiagnostics(document.uri);
        if (diagnostics.length === 0) {
            return;
        }

        const result = await this.lspBridge.exportDiagnostics({
            format: 'json',
            privacyLevel: config.get('privacyLevel') as string,
            includeContext: false,
            contextLines: 0,
            files: [document.fileName]
        });

        // Store in workspace state for history
        const key = `lsp-bridge.history.${document.fileName}`;
        const history = JSON.parse(result);
        // This would normally append to history, not replace
        vscode.workspace.globalState.update(key, history);
    }

    async startWatching(): Promise<vscode.Disposable> {
        const outputChannel = vscode.window.createOutputChannel('LSP Bridge Watch');
        outputChannel.show();

        const process = await this.lspBridge.watchDiagnostics((data) => {
            outputChannel.append(data);
        });

        return new vscode.Disposable(() => {
            process.kill();
            outputChannel.dispose();
        });
    }

    private async getFilesForScope(scope: string): Promise<string[] | undefined> {
        switch (scope) {
            case 'Current File':
                const activeEditor = vscode.window.activeTextEditor;
                return activeEditor ? [activeEditor.document.fileName] : undefined;
            
            case 'Open Files':
                return vscode.window.visibleTextEditors.map(e => e.document.fileName);
            
            case 'Workspace':
            case 'Errors Only':
                return undefined; // All files
            
            default:
                return undefined;
        }
    }

    private countDiagnostics(result: string): number {
        try {
            // Try to parse as JSON first
            const parsed = JSON.parse(result);
            if (parsed.diagnostics) {
                return parsed.diagnostics.length;
            }
            
            // For markdown format, count bullet points
            const matches = result.match(/^[\*\-]\s+/gm);
            return matches ? matches.length : 0;
        } catch {
            // For other formats, just count lines with diagnostic patterns
            const lines = result.split('\n');
            return lines.filter(l => l.includes('Error:') || l.includes('Warning:')).length;
        }
    }
}