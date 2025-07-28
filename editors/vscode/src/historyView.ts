import * as vscode from 'vscode';
import { LspBridgeProvider } from './lspBridgeProvider';

export class HistoryView {
    private panel: vscode.WebviewPanel | undefined;

    constructor(private lspBridge: LspBridgeProvider) {}

    async show(): Promise<void> {
        if (!this.panel) {
            this.panel = vscode.window.createWebviewPanel(
                'lspBridgeHistory',
                'LSP Bridge History',
                vscode.ViewColumn.Two,
                {
                    enableScripts: true,
                    retainContextWhenHidden: true
                }
            );

            this.panel.onDidDispose(() => {
                this.panel = undefined;
            });
        }

        this.panel.webview.html = await this.getHtmlContent();
        this.panel.reveal();
    }

    private async getHtmlContent(): Promise<string> {
        const trends = await this.lspBridge.getHistory(24);
        const hotSpots = await this.lspBridge.getHotSpots(10);

        return `<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>LSP Bridge History</title>
    <style>
        body {
            font-family: var(--vscode-font-family);
            color: var(--vscode-foreground);
            background-color: var(--vscode-editor-background);
            padding: 20px;
            line-height: 1.6;
        }
        
        h1, h2 {
            color: var(--vscode-foreground);
            border-bottom: 1px solid var(--vscode-widget-border);
            padding-bottom: 10px;
            margin-bottom: 20px;
        }
        
        .metric {
            display: inline-block;
            margin: 10px;
            padding: 15px;
            background-color: var(--vscode-editor-inactiveSelectionBackground);
            border-radius: 5px;
            min-width: 150px;
            text-align: center;
        }
        
        .metric-value {
            font-size: 2em;
            font-weight: bold;
            color: var(--vscode-textLink-foreground);
        }
        
        .metric-label {
            font-size: 0.9em;
            color: var(--vscode-descriptionForeground);
            margin-top: 5px;
        }
        
        .hot-spot {
            margin: 10px 0;
            padding: 10px;
            background-color: var(--vscode-editor-selectionBackground);
            border-radius: 3px;
        }
        
        .file-path {
            font-family: var(--vscode-editor-font-family);
            color: var(--vscode-textLink-foreground);
            cursor: pointer;
        }
        
        .file-path:hover {
            text-decoration: underline;
        }
        
        .error-count {
            color: var(--vscode-errorForeground);
        }
        
        .warning-count {
            color: var(--vscode-warningForeground);
        }
        
        .trend-up {
            color: var(--vscode-errorForeground);
        }
        
        .trend-down {
            color: var(--vscode-successBackground);
        }
        
        .chart-container {
            margin: 20px 0;
            height: 200px;
            background-color: var(--vscode-editor-inactiveSelectionBackground);
            border-radius: 5px;
            display: flex;
            align-items: center;
            justify-content: center;
            color: var(--vscode-descriptionForeground);
        }
    </style>
</head>
<body>
    <h1>Diagnostic History</h1>
    
    <div class="metrics">
        <div class="metric">
            <div class="metric-value">${trends.health_score ? Math.round(trends.health_score * 100) : 0}%</div>
            <div class="metric-label">Health Score</div>
        </div>
        
        <div class="metric">
            <div class="metric-value">${trends.error_velocity || 0}</div>
            <div class="metric-label">Errors/Hour</div>
        </div>
        
        <div class="metric">
            <div class="metric-value">${trends.warning_velocity || 0}</div>
            <div class="metric-label">Warnings/Hour</div>
        </div>
        
        <div class="metric">
            <div class="metric-value ${this.getTrendClass(trends.trend_direction)}">
                ${this.getTrendIcon(trends.trend_direction)} ${trends.trend_direction || 'Stable'}
            </div>
            <div class="metric-label">Trend</div>
        </div>
    </div>
    
    <h2>Hot Spots</h2>
    <div class="hot-spots">
        ${hotSpots.map((spot: any) => `
            <div class="hot-spot">
                <div class="file-path" onclick="openFile('${spot.file_path}')">${spot.file_path}</div>
                <div>
                    <span class="error-count">Errors: ${spot.last_error_count || 0}</span> | 
                    <span class="warning-count">Warnings: ${spot.last_warning_count || 0}</span>
                </div>
            </div>
        `).join('')}
    </div>
    
    <h2>Trend Chart</h2>
    <div class="chart-container">
        <div>Chart visualization would go here</div>
    </div>
    
    <script>
        const vscode = acquireVsCodeApi();
        
        function openFile(path) {
            vscode.postMessage({
                command: 'openFile',
                path: path
            });
        }
    </script>
</body>
</html>`;
    }

    private getTrendClass(trend: string): string {
        if (!trend) return '';
        return trend.toLowerCase().includes('improv') ? 'trend-down' : 'trend-up';
    }

    private getTrendIcon(trend: string): string {
        if (!trend) return '→';
        if (trend.toLowerCase().includes('improv')) return '↓';
        if (trend.toLowerCase().includes('degrad')) return '↑';
        return '→';
    }
}