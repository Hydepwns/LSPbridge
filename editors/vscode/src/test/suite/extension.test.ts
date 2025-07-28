import * as assert from 'assert';
import * as vscode from 'vscode';

suite('Extension Test Suite', () => {
    vscode.window.showInformationMessage('Start all tests.');

    test('Extension should be present', () => {
        assert.ok(vscode.extensions.getExtension('lsp-bridge.lsp-bridge'));
    });

    test('Should register all commands', async () => {
        const commands = await vscode.commands.getCommands();
        
        assert.ok(commands.includes('lsp-bridge.exportDiagnostics'));
        assert.ok(commands.includes('lsp-bridge.exportToClipboard'));
        assert.ok(commands.includes('lsp-bridge.watchDiagnostics'));
        assert.ok(commands.includes('lsp-bridge.stopWatching'));
        assert.ok(commands.includes('lsp-bridge.showHistory'));
        assert.ok(commands.includes('lsp-bridge.applyQuickFixes'));
    });

    test('Configuration should have default values', () => {
        const config = vscode.workspace.getConfiguration('lsp-bridge');
        
        assert.strictEqual(config.get('exportFormat'), 'claude');
        assert.strictEqual(config.get('privacyLevel'), 'default');
        assert.strictEqual(config.get('includeContext'), true);
        assert.strictEqual(config.get('contextLines'), 3);
        assert.strictEqual(config.get('autoExportOnSave'), false);
        assert.strictEqual(config.get('quickFixThreshold'), 0.9);
    });
});