import * as vscode from 'vscode';
import { spawn, ChildProcess } from 'child_process';
import * as path from 'path';

export class LspBridgeProvider {
    private lspBridgePath: string;
    private outputChannel: vscode.OutputChannel;

    constructor(private context: vscode.ExtensionContext) {
        const config = vscode.workspace.getConfiguration('lsp-bridge');
        this.lspBridgePath = config.get('executablePath') || 'lsp-bridge';
        this.outputChannel = vscode.window.createOutputChannel('LSP Bridge');
    }

    async executeCommand(args: string[]): Promise<string> {
        return new Promise((resolve, reject) => {
            const workspaceFolder = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
            
            const process = spawn(this.lspBridgePath, args, {
                cwd: workspaceFolder,
                env: { ...process.env, NO_COLOR: '1' }
            });

            let stdout = '';
            let stderr = '';

            process.stdout.on('data', (data) => {
                const text = data.toString();
                stdout += text;
                this.outputChannel.append(text);
            });

            process.stderr.on('data', (data) => {
                const text = data.toString();
                stderr += text;
                this.outputChannel.append(text);
            });

            process.on('close', (code) => {
                if (code === 0) {
                    resolve(stdout);
                } else {
                    reject(new Error(`LSP Bridge exited with code ${code}: ${stderr}`));
                }
            });

            process.on('error', (err) => {
                reject(new Error(`Failed to start LSP Bridge: ${err.message}`));
            });
        });
    }

    async exportDiagnostics(options: {
        format: string;
        privacyLevel: string;
        includeContext: boolean;
        contextLines: number;
        outputPath?: string;
        errorsOnly?: boolean;
        files?: string[];
    }): Promise<string> {
        const args = ['export', '--format', options.format];

        if (options.privacyLevel) {
            args.push('--privacy', options.privacyLevel);
        }

        if (options.includeContext) {
            args.push('--include-context');
            args.push('--context-lines', options.contextLines.toString());
        }

        if (options.errorsOnly) {
            args.push('--errors-only');
        }

        if (options.files && options.files.length > 0) {
            args.push('--files', options.files.join(','));
        }

        if (options.outputPath) {
            args.push('--output', options.outputPath);
        }

        // Convert VS Code diagnostics to LSP Bridge format
        const diagnostics = this.collectDiagnostics(options.files);
        const input = JSON.stringify(diagnostics);

        return this.executeCommandWithInput(args, input);
    }

    async watchDiagnostics(callback: (data: string) => void): Promise<ChildProcess> {
        const config = vscode.workspace.getConfiguration('lsp-bridge');
        const format = config.get('exportFormat') as string;
        const privacyLevel = config.get('privacyLevel') as string;

        const args = ['watch', '--format', format, '--privacy', privacyLevel];
        
        const workspaceFolder = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
        
        const process = spawn(this.lspBridgePath, args, {
            cwd: workspaceFolder,
            env: { ...process.env, NO_COLOR: '1' }
        });

        process.stdout.on('data', (data) => {
            callback(data.toString());
        });

        process.stderr.on('data', (data) => {
            this.outputChannel.append(data.toString());
        });

        return process;
    }

    async getHistory(hours: number = 24): Promise<any> {
        const args = ['history', 'trends', '--hours', hours.toString(), '--format', 'json'];
        const result = await this.executeCommand(args);
        return JSON.parse(result);
    }

    async getHotSpots(limit: number = 10): Promise<any> {
        const args = ['history', 'hot-spots', '--limit', limit.toString(), '--format', 'json'];
        const result = await this.executeCommand(args);
        return JSON.parse(result);
    }

    async applyQuickFixes(threshold: number = 0.9, dryRun: boolean = false): Promise<any> {
        const args = ['quick-fix', 'apply', '--threshold', threshold.toString()];
        
        if (dryRun) {
            args.push('--dry-run');
        }

        const diagnostics = this.collectDiagnostics();
        const input = JSON.stringify(diagnostics);
        const result = await this.executeCommandWithInput(args, input);
        
        // Parse the output to get applied fixes
        return this.parseQuickFixResult(result);
    }

    private collectDiagnostics(files?: string[]): any {
        const diagnosticsMap = vscode.languages.getDiagnostics();
        const result: any = {
            source: 'vscode',
            timestamp: new Date().toISOString(),
            diagnostics: []
        };

        for (const [uri, diagnostics] of diagnosticsMap) {
            // Filter by files if specified
            if (files && !files.some(f => uri.fsPath.includes(f))) {
                continue;
            }

            for (const diagnostic of diagnostics) {
                result.diagnostics.push({
                    file: uri.fsPath,
                    message: diagnostic.message,
                    severity: this.convertSeverity(diagnostic.severity),
                    range: {
                        start: {
                            line: diagnostic.range.start.line + 1,
                            character: diagnostic.range.start.character
                        },
                        end: {
                            line: diagnostic.range.end.line + 1,
                            character: diagnostic.range.end.character
                        }
                    },
                    code: diagnostic.code?.toString(),
                    source: diagnostic.source
                });
            }
        }

        return result;
    }

    private convertSeverity(severity: vscode.DiagnosticSeverity): string {
        switch (severity) {
            case vscode.DiagnosticSeverity.Error:
                return 'Error';
            case vscode.DiagnosticSeverity.Warning:
                return 'Warning';
            case vscode.DiagnosticSeverity.Information:
                return 'Information';
            case vscode.DiagnosticSeverity.Hint:
                return 'Hint';
            default:
                return 'Information';
        }
    }

    private async executeCommandWithInput(args: string[], input: string): Promise<string> {
        return new Promise((resolve, reject) => {
            const workspaceFolder = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
            
            const process = spawn(this.lspBridgePath, args, {
                cwd: workspaceFolder,
                env: { ...process.env, NO_COLOR: '1' }
            });

            let stdout = '';
            let stderr = '';

            process.stdout.on('data', (data) => {
                stdout += data.toString();
            });

            process.stderr.on('data', (data) => {
                stderr += data.toString();
            });

            process.on('close', (code) => {
                if (code === 0) {
                    resolve(stdout);
                } else {
                    reject(new Error(`LSP Bridge exited with code ${code}: ${stderr}`));
                }
            });

            process.on('error', (err) => {
                reject(new Error(`Failed to start LSP Bridge: ${err.message}`));
            });

            // Send input
            process.stdin.write(input);
            process.stdin.end();
        });
    }

    private parseQuickFixResult(output: string): any {
        // Parse the output to extract fix information
        // This is a simplified version - actual implementation would be more robust
        const lines = output.split('\n');
        const fixes = [];
        
        for (const line of lines) {
            if (line.includes('Successfully applied:')) {
                const match = line.match(/Successfully applied: (\d+)/);
                if (match) {
                    return {
                        success: true,
                        appliedCount: parseInt(match[1]),
                        fixes
                    };
                }
            }
        }
        
        return {
            success: false,
            appliedCount: 0,
            fixes: []
        };
    }
}