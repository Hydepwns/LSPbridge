import * as vscode from 'vscode';
import { LspBridgeProvider } from './lspBridgeProvider';

export class QuickFixProvider {
    constructor(private lspBridge: LspBridgeProvider) {}

    async applyFixes(): Promise<void> {
        const config = vscode.workspace.getConfiguration('lsp-bridge');
        const threshold = config.get('quickFixThreshold') as number;

        // First, do a dry run to show what will be fixed
        const dryRunResult = await this.lspBridge.applyQuickFixes(threshold, true);
        
        if (!dryRunResult.fixes || dryRunResult.fixes.length === 0) {
            vscode.window.showInformationMessage('No fixes available with sufficient confidence');
            return;
        }

        // Show preview to user
        const message = `Found ${dryRunResult.fixes.length} fixes with confidence >= ${threshold}. Apply them?`;
        const result = await vscode.window.showInformationMessage(
            message,
            'Apply Fixes',
            'Show Details',
            'Cancel'
        );

        if (result === 'Cancel' || !result) {
            return;
        }

        if (result === 'Show Details') {
            await this.showFixDetails(dryRunResult);
            return;
        }

        // Apply the fixes
        const applyResult = await this.lspBridge.applyQuickFixes(threshold, false);
        
        if (applyResult.success) {
            vscode.window.showInformationMessage(
                `Successfully applied ${applyResult.appliedCount} fixes`
            );
            
            // Refresh diagnostics
            await this.refreshDiagnostics();
        } else {
            vscode.window.showErrorMessage('Failed to apply fixes');
        }
    }

    private async showFixDetails(dryRunResult: any): Promise<void> {
        const quickPick = vscode.window.createQuickPick();
        quickPick.title = 'Available Quick Fixes';
        quickPick.placeholder = 'Select fixes to apply';
        quickPick.canSelectMany = true;
        
        quickPick.items = dryRunResult.fixes.map((fix: any) => ({
            label: fix.message,
            description: `${fix.file}:${fix.line}`,
            detail: `Confidence: ${Math.round(fix.confidence * 100)}%`,
            picked: fix.confidence >= 0.9
        }));

        quickPick.onDidAccept(async () => {
            const selected = quickPick.selectedItems;
            if (selected.length > 0) {
                // Apply selected fixes
                // This would require a more sophisticated API to apply specific fixes
                vscode.window.showInformationMessage(
                    `Would apply ${selected.length} selected fixes`
                );
            }
            quickPick.dispose();
        });

        quickPick.show();
    }

    private async refreshDiagnostics(): Promise<void> {
        // Force VS Code to refresh diagnostics
        // This is typically done by the language server automatically
        // but we can trigger a document change event if needed
        const activeEditor = vscode.window.activeTextEditor;
        if (activeEditor) {
            // Save and reopen to refresh
            await activeEditor.document.save();
            const uri = activeEditor.document.uri;
            await vscode.commands.executeCommand('workbench.action.closeActiveEditor');
            await vscode.window.showTextDocument(uri);
        }
    }
}

// Register as a VS Code code action provider for quick fixes
export class LspBridgeCodeActionProvider implements vscode.CodeActionProvider {
    constructor(private quickFixProvider: QuickFixProvider) {}

    provideCodeActions(
        document: vscode.TextDocument,
        range: vscode.Range | vscode.Selection,
        context: vscode.CodeActionContext,
        token: vscode.CancellationToken
    ): vscode.ProviderResult<(vscode.CodeAction | vscode.Command)[]> {
        const actions: vscode.CodeAction[] = [];

        // For each diagnostic in the context
        for (const diagnostic of context.diagnostics) {
            const action = new vscode.CodeAction(
                `Apply LSP Bridge fix for: ${diagnostic.message}`,
                vscode.CodeActionKind.QuickFix
            );
            
            action.command = {
                title: 'Apply LSP Bridge Fix',
                command: 'lsp-bridge.applyQuickFixes'
            };
            
            action.diagnostics = [diagnostic];
            action.isPreferred = true;
            
            actions.push(action);
        }

        return actions;
    }
}