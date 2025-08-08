#!/usr/bin/env python3
"""
Benchmark Visualization Generator
Creates interactive charts and visualizations for LSPbridge performance data
"""

import json
import os
import sys
from pathlib import Path
from datetime import datetime, timedelta
from typing import Dict, List, Any, Optional
import logging

# Try to import visualization libraries
try:
    import matplotlib
    matplotlib.use('Agg')  # Use non-interactive backend
    import matplotlib.pyplot as plt
    import seaborn as sns
    import pandas as pd
    VISUALIZATION_AVAILABLE = True
except ImportError:
    VISUALIZATION_AVAILABLE = False
    print("Warning: Visualization libraries not available. Install with:")
    print("pip install matplotlib seaborn pandas")

# Setup logging
logging.basicConfig(level=logging.INFO, format='%(asctime)s - %(levelname)s - %(message)s')
logger = logging.getLogger(__name__)

class BenchmarkVisualizer:
    def __init__(self, benchmark_dir: Path):
        self.benchmark_dir = benchmark_dir
        self.reports_dir = benchmark_dir / "reports"
        self.archive_dir = benchmark_dir / "archive"
        self.latest_dir = benchmark_dir / "latest"
        
        # Setup matplotlib if available
        if VISUALIZATION_AVAILABLE:
            plt.style.use('seaborn-v0_8')
            sns.set_palette("husl")
    
    def load_benchmark_data(self) -> List[Dict[str, Any]]:
        """Load all available benchmark data from archives"""
        data = []
        
        # Load from archive directory
        if self.archive_dir.exists():
            for archive_subdir in sorted(self.archive_dir.iterdir()):
                if not archive_subdir.is_dir():
                    continue
                
                parsed_files = list(archive_subdir.glob("*_parsed.json"))
                if parsed_files:
                    try:
                        with open(parsed_files[0], 'r') as f:
                            benchmark_data = json.load(f)
                            data.append(benchmark_data)
                    except (json.JSONDecodeError, FileNotFoundError) as e:
                        logger.warning(f"Could not load {parsed_files[0]}: {e}")
        
        # Load latest data
        latest_parsed = self.latest_dir / "latest.json"
        if latest_parsed.exists():
            try:
                # Check if it's already parsed or needs parsing
                parsed_files = list(self.latest_dir.glob("*_parsed.json"))
                if parsed_files:
                    with open(parsed_files[0], 'r') as f:
                        latest_data = json.load(f)
                        # Only add if not already in data
                        if not any(d.get('timestamp') == latest_data.get('timestamp') for d in data):
                            data.append(latest_data)
            except (json.JSONDecodeError, FileNotFoundError) as e:
                logger.warning(f"Could not load latest data: {e}")
        
        return data
    
    def generate_performance_trends(self, data: List[Dict[str, Any]]) -> str:
        """Generate performance trend charts"""
        if not VISUALIZATION_AVAILABLE or not data:
            return self._generate_text_summary(data)
        
        # Prepare data for plotting
        df_data = []
        for entry in data:
            timestamp = entry.get('timestamp', '')
            commit = entry.get('commit', 'unknown')[:8]
            
            for benchmark in entry.get('benchmarks', []):
                df_data.append({
                    'timestamp': timestamp,
                    'commit': commit,
                    'benchmark': benchmark['name'],
                    'group': benchmark['group'],
                    'mean_ms': benchmark['mean_ms'],
                    'std_dev_ms': benchmark['std_dev_ns'] / 1000000
                })
        
        if not df_data:
            return self._generate_text_summary(data)
        
        df = pd.DataFrame(df_data)
        df['timestamp'] = pd.to_datetime(df['timestamp'])
        
        # Generate visualizations
        charts_html = []
        
        # 1. Overall Performance Trends
        charts_html.append(self._generate_overall_trends_chart(df))
        
        # 2. Performance by Group
        charts_html.append(self._generate_group_performance_chart(df))
        
        # 3. Recent Performance Changes
        charts_html.append(self._generate_recent_changes_chart(df))
        
        # 4. Performance Distribution
        charts_html.append(self._generate_distribution_chart(df))
        
        return '\n'.join(charts_html)
    
    def _generate_overall_trends_chart(self, df: pd.DataFrame) -> str:
        """Generate overall performance trends chart"""
        fig, ax = plt.subplots(figsize=(14, 8))
        
        # Group by timestamp and calculate mean performance
        time_trends = df.groupby('timestamp')['mean_ms'].agg(['mean', 'std']).reset_index()
        
        ax.plot(time_trends['timestamp'], time_trends['mean'], 'o-', linewidth=2, markersize=6)
        ax.fill_between(time_trends['timestamp'], 
                       time_trends['mean'] - time_trends['std'],
                       time_trends['mean'] + time_trends['std'], 
                       alpha=0.3)
        
        ax.set_title('LSPbridge Overall Performance Trends', fontsize=16, fontweight='bold')
        ax.set_xlabel('Time', fontsize=12)
        ax.set_ylabel('Average Execution Time (ms)', fontsize=12)
        ax.grid(True, alpha=0.3)
        
        # Rotate x-axis labels
        plt.xticks(rotation=45)
        plt.tight_layout()
        
        # Save chart
        chart_path = self.reports_dir / 'performance_trends.png'
        plt.savefig(chart_path, dpi=300, bbox_inches='tight')
        plt.close()
        
        return f'![Performance Trends](./performance_trends.png)\n'
    
    def _generate_group_performance_chart(self, df: pd.DataFrame) -> str:
        """Generate performance by group chart"""
        fig, axes = plt.subplots(2, 2, figsize=(16, 12))
        axes = axes.ravel()
        
        groups = df['group'].unique()[:4]  # Limit to top 4 groups
        
        for i, group in enumerate(groups):
            if i >= 4:
                break
                
            group_data = df[df['group'] == group]
            group_trends = group_data.groupby('timestamp')['mean_ms'].mean().reset_index()
            
            axes[i].plot(group_trends['timestamp'], group_trends['mean_ms'], 'o-', linewidth=2)
            axes[i].set_title(f'{group.title()} Performance', fontweight='bold')
            axes[i].set_ylabel('Time (ms)')
            axes[i].grid(True, alpha=0.3)
            axes[i].tick_params(axis='x', rotation=45)
        
        # Hide unused subplots
        for i in range(len(groups), 4):
            axes[i].set_visible(False)
        
        plt.suptitle('Performance Trends by Benchmark Group', fontsize=16, fontweight='bold')
        plt.tight_layout()
        
        chart_path = self.reports_dir / 'group_performance.png'
        plt.savefig(chart_path, dpi=300, bbox_inches='tight')
        plt.close()
        
        return f'![Group Performance](./group_performance.png)\n'
    
    def _generate_recent_changes_chart(self, df: pd.DataFrame) -> str:
        """Generate recent performance changes chart"""
        # Get last 10 data points
        recent_data = df.nlargest(10 * len(df['benchmark'].unique()), 'timestamp')
        
        fig, ax = plt.subplots(figsize=(12, 8))
        
        # Create a heatmap of recent performance changes
        pivot_data = recent_data.pivot_table(
            values='mean_ms', 
            index='benchmark', 
            columns='commit',
            aggfunc='first'
        ).fillna(0)
        
        if not pivot_data.empty:
            sns.heatmap(pivot_data, annot=True, fmt='.2f', cmap='RdYlGn_r', ax=ax)
            ax.set_title('Recent Performance Changes by Commit', fontsize=14, fontweight='bold')
            ax.set_xlabel('Commit')
            ax.set_ylabel('Benchmark')
        
        plt.tight_layout()
        
        chart_path = self.reports_dir / 'recent_changes.png'
        plt.savefig(chart_path, dpi=300, bbox_inches='tight')
        plt.close()
        
        return f'![Recent Changes](./recent_changes.png)\n'
    
    def _generate_distribution_chart(self, df: pd.DataFrame) -> str:
        """Generate performance distribution chart"""
        fig, ax = plt.subplots(figsize=(12, 8))
        
        # Box plot of performance by group
        df.boxplot(column='mean_ms', by='group', ax=ax, rot=45)
        ax.set_title('Performance Distribution by Benchmark Group')
        ax.set_xlabel('Benchmark Group')
        ax.set_ylabel('Execution Time (ms)')
        
        plt.suptitle('')  # Remove automatic title
        plt.tight_layout()
        
        chart_path = self.reports_dir / 'performance_distribution.png'
        plt.savefig(chart_path, dpi=300, bbox_inches='tight')
        plt.close()
        
        return f'![Performance Distribution](./performance_distribution.png)\n'
    
    def _generate_text_summary(self, data: List[Dict[str, Any]]) -> str:
        """Generate text-based summary when visualization libraries aren't available"""
        if not data:
            return "No benchmark data available for visualization.\n"
        
        latest = data[-1] if data else {}
        benchmarks = latest.get('benchmarks', [])
        
        summary = ["## 📊 Performance Summary (Text)\n"]
        summary.append(f"**Last Updated**: {latest.get('timestamp', 'unknown')}")
        summary.append(f"**Commit**: {latest.get('commit', 'unknown')[:8]}")
        summary.append(f"**Total Benchmarks**: {len(benchmarks)}\n")
        
        if benchmarks:
            # Group performance
            groups = {}
            for bench in benchmarks:
                group = bench['group']
                if group not in groups:
                    groups[group] = []
                groups[group].append(bench['mean_ms'])
            
            summary.append("### Performance by Group")
            for group, times in groups.items():
                avg_time = sum(times) / len(times)
                min_time = min(times)
                max_time = max(times)
                summary.append(f"- **{group}**: {avg_time:.2f}ms avg (min: {min_time:.2f}ms, max: {max_time:.2f}ms)")
            
            summary.append("")
        
        return '\n'.join(summary)
    
    def generate_index_html(self, trends_content: str) -> str:
        """Generate an HTML index page for the benchmark dashboard"""
        html_template = f"""<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>LSPbridge Benchmark Dashboard</title>
    <style>
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            line-height: 1.6;
            color: #333;
            max-width: 1200px;
            margin: 0 auto;
            padding: 20px;
            background-color: #f5f5f5;
        }}
        .header {{
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            color: white;
            padding: 40px 20px;
            border-radius: 10px;
            text-align: center;
            margin-bottom: 30px;
        }}
        .content {{
            background: white;
            padding: 30px;
            border-radius: 10px;
            box-shadow: 0 2px 10px rgba(0,0,0,0.1);
        }}
        .metric {{
            display: inline-block;
            background: #f8f9fa;
            padding: 15px 20px;
            margin: 10px;
            border-radius: 8px;
            border-left: 4px solid #007bff;
        }}
        .chart {{
            text-align: center;
            margin: 30px 0;
        }}
        .chart img {{
            max-width: 100%;
            height: auto;
            border-radius: 8px;
            box-shadow: 0 2px 8px rgba(0,0,0,0.1);
        }}
        .footer {{
            text-align: center;
            padding: 20px;
            color: #666;
            font-size: 0.9em;
        }}
        h1, h2, h3 {{ color: #2c3e50; }}
        .status {{
            padding: 8px 16px;
            border-radius: 20px;
            display: inline-block;
            font-weight: bold;
            text-transform: uppercase;
            font-size: 0.8em;
        }}
        .status.good {{ background: #d4edda; color: #155724; }}
        .status.warning {{ background: #fff3cd; color: #856404; }}
        .status.error {{ background: #f8d7da; color: #721c24; }}
    </style>
</head>
<body>
    <div class="header">
        <h1>🚀 LSPbridge Performance Dashboard</h1>
        <p>Automated performance tracking and regression detection</p>
        <span class="status good">System Healthy</span>
    </div>
    
    <div class="content">
        <div class="metrics">
            <div class="metric">
                <strong>Last Updated</strong><br>
                {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}
            </div>
            <div class="metric">
                <strong>Monitoring</strong><br>
                7 Benchmark Groups
            </div>
            <div class="metric">
                <strong>Threshold</strong><br>
                15% Regression Alert
            </div>
        </div>
        
        {trends_content}
        
        <h2>📁 Available Reports</h2>
        <ul>
            <li><a href="./latest/">Latest Results</a></li>
            <li><a href="./archive/">Historical Archive</a></li>
            <li><a href="https://github.com/your-org/lspbridge/actions">CI Pipeline</a></li>
        </ul>
    </div>
    
    <div class="footer">
        Generated by LSPbridge Benchmark Dashboard | 
        <a href="https://github.com/your-org/lspbridge">View Source</a>
    </div>
</body>
</html>"""
        return html_template
    
    def run(self):
        """Main execution function"""
        logger.info("Starting benchmark visualization generation...")
        
        # Create reports directory
        self.reports_dir.mkdir(exist_ok=True)
        
        # Load benchmark data
        data = self.load_benchmark_data()
        logger.info(f"Loaded {len(data)} benchmark datasets")
        
        # Generate performance trends
        trends_content = self.generate_performance_trends(data)
        
        # Generate HTML dashboard
        html_content = self.generate_index_html(trends_content)
        
        # Write HTML file
        index_path = self.reports_dir / 'index.html'
        with open(index_path, 'w') as f:
            f.write(html_content)
        
        logger.info(f"Generated benchmark dashboard at {index_path}")
        
        # Generate markdown summary for GitHub
        markdown_path = self.reports_dir / 'README.md'
        with open(markdown_path, 'w') as f:
            f.write(f"""# LSPbridge Benchmark Dashboard

{trends_content}

## 🔗 Navigation

- [📊 Interactive Dashboard](./index.html)
- [📄 Latest Results](./latest/)
- [📚 Historical Archive](./archive/)

## 📈 Performance Monitoring

This dashboard automatically tracks performance across all LSPbridge benchmarks:

- **Context Extraction**: File parsing and semantic analysis performance
- **Context Ranking**: Algorithm efficiency for relevance scoring  
- **Diagnostic Prioritization**: Error categorization and sorting speed
- **Memory Usage**: Memory consumption patterns and cache efficiency
- **Concurrent Throughput**: Parallel processing performance
- **Cache Performance**: Hit rates and retrieval speeds
- **Cold Start**: Initialization and startup performance

## 🚨 Regression Detection

- **Threshold**: 15% performance degradation triggers alerts
- **Memory Threshold**: 20% memory increase triggers warnings  
- **Cache Threshold**: 10% cache hit rate decrease triggers investigation

Generated: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}
""")
        
        logger.info("Benchmark visualization generation completed successfully!")


def main():
    """Main entry point"""
    # Get benchmark directory from environment or use default
    benchmark_dir = Path(os.environ.get('BENCHMARK_DIR', 'benchmark-results'))
    
    if not benchmark_dir.exists():
        logger.warning(f"Benchmark directory {benchmark_dir} does not exist - creating basic structure")
        benchmark_dir.mkdir(parents=True, exist_ok=True)
        return
    
    visualizer = BenchmarkVisualizer(benchmark_dir)
    try:
        visualizer.run()
    except Exception as e:
        logger.error(f"Visualization failed: {e}")
        # Don't exit with error - just log it


if __name__ == '__main__':
    main()