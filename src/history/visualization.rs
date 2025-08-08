use crate::history::{HotSpot, TimeSeriesPoint, TrendAnalysis};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

/// Visualization data formats for different charting libraries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualizationData {
    pub charts: Vec<ChartData>,
    pub metadata: VisualizationMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualizationMetadata {
    pub generated_at: SystemTime,
    pub title: String,
    pub description: String,
    pub time_range: TimeRange,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRange {
    pub start: SystemTime,
    pub end: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ChartData {
    TimeSeries(TimeSeriesChart),
    Bar(BarChart),
    Pie(PieChart),
    Heatmap(HeatmapChart),
    Scatter(ScatterChart),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeriesChart {
    pub title: String,
    pub x_label: String,
    pub y_label: String,
    pub series: Vec<Series>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Series {
    pub name: String,
    pub data: Vec<DataPoint>,
    pub color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataPoint {
    pub x: f64, // Unix timestamp
    pub y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BarChart {
    pub title: String,
    pub x_label: String,
    pub y_label: String,
    pub categories: Vec<String>,
    pub values: Vec<f64>,
    pub colors: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PieChart {
    pub title: String,
    pub segments: Vec<PieSegment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PieSegment {
    pub label: String,
    pub value: f64,
    pub color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeatmapChart {
    pub title: String,
    pub x_labels: Vec<String>,
    pub y_labels: Vec<String>,
    pub data: Vec<Vec<f64>>,
    pub color_scale: Option<ColorScale>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorScale {
    pub min_color: String,
    pub max_color: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScatterChart {
    pub title: String,
    pub x_label: String,
    pub y_label: String,
    pub points: Vec<ScatterPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScatterPoint {
    pub x: f64,
    pub y: f64,
    pub label: Option<String>,
    pub size: Option<f64>,
    pub color: Option<String>,
}

pub struct VisualizationExporter;

impl VisualizationExporter {
    /// Export time series data for visualization
    pub fn export_time_series(
        points: &[TimeSeriesPoint],
        title: &str,
    ) -> Result<VisualizationData> {
        let mut error_data = Vec::new();
        let mut warning_data = Vec::new();

        for point in points {
            let timestamp = point
                .timestamp
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs() as f64;

            error_data.push(DataPoint {
                x: timestamp,
                y: point.total_errors as f64,
            });

            warning_data.push(DataPoint {
                x: timestamp,
                y: point.total_warnings as f64,
            });
        }

        let time_series_chart = TimeSeriesChart {
            title: title.to_string(),
            x_label: "Time".to_string(),
            y_label: "Count".to_string(),
            series: vec![
                Series {
                    name: "Errors".to_string(),
                    data: error_data,
                    color: Some("#ff6b6b".to_string()),
                },
                Series {
                    name: "Warnings".to_string(),
                    data: warning_data,
                    color: Some("#ffd93d".to_string()),
                },
            ],
        };

        let time_range = if let (Some(first), Some(last)) = (points.first(), points.last()) {
            TimeRange {
                start: first.timestamp,
                end: last.timestamp,
            }
        } else {
            TimeRange {
                start: SystemTime::now(),
                end: SystemTime::now(),
            }
        };

        Ok(VisualizationData {
            charts: vec![ChartData::TimeSeries(time_series_chart)],
            metadata: VisualizationMetadata {
                generated_at: SystemTime::now(),
                title: title.to_string(),
                description: "Diagnostic trends over time".to_string(),
                time_range,
            },
        })
    }

    /// Export hot spots as a bar chart
    pub fn export_hot_spots(hot_spots: &[HotSpot]) -> Result<VisualizationData> {
        let categories: Vec<String> = hot_spots
            .iter()
            .map(|hs| {
                let file_name = hs
                    .file_path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| hs.file_path.to_string_lossy().to_string());
                file_name
            })
            .collect();

        let values: Vec<f64> = hot_spots.iter().map(|hs| hs.score as f64).collect();

        let colors: Vec<String> = hot_spots
            .iter()
            .map(|hs| {
                match hs.trend {
                    crate::history::TrendDirection::Improving => "#4ecdc4",
                    crate::history::TrendDirection::Stable => "#f7b731",
                    crate::history::TrendDirection::Degrading => "#ff6b6b",
                }
                .to_string()
            })
            .collect();

        let bar_chart = BarChart {
            title: "Diagnostic Hot Spots".to_string(),
            x_label: "File".to_string(),
            y_label: "Problem Score".to_string(),
            categories,
            values,
            colors: Some(colors),
        };

        Ok(VisualizationData {
            charts: vec![ChartData::Bar(bar_chart)],
            metadata: VisualizationMetadata {
                generated_at: SystemTime::now(),
                title: "Hot Spots Analysis".to_string(),
                description: "Files with the most diagnostic issues".to_string(),
                time_range: TimeRange {
                    start: SystemTime::now(),
                    end: SystemTime::now(),
                },
            },
        })
    }

    /// Export trend analysis as multiple charts
    pub fn export_trend_analysis(trends: &TrendAnalysis) -> Result<VisualizationData> {
        let mut charts = Vec::new();

        // 1. Health score gauge (represented as a single-value pie chart)
        let health_pie = PieChart {
            title: "Project Health Score".to_string(),
            segments: vec![
                PieSegment {
                    label: "Health".to_string(),
                    value: trends.health_score as f64,
                    color: Some(
                        if trends.health_score > 0.8 {
                            "#4ecdc4"
                        } else if trends.health_score > 0.5 {
                            "#f7b731"
                        } else {
                            "#ff6b6b"
                        }
                        .to_string(),
                    ),
                },
                PieSegment {
                    label: "Issues".to_string(),
                    value: (1.0 - trends.health_score) as f64,
                    color: Some("#e0e0e0".to_string()),
                },
            ],
        };
        charts.push(ChartData::Pie(health_pie));

        // 2. Velocity scatter plot
        let velocity_scatter = ScatterChart {
            title: "Error vs Warning Velocity".to_string(),
            x_label: "Error Velocity (per hour)".to_string(),
            y_label: "Warning Velocity (per hour)".to_string(),
            points: vec![ScatterPoint {
                x: trends.error_velocity as f64,
                y: trends.warning_velocity as f64,
                label: Some("Current".to_string()),
                size: Some(10.0),
                color: Some("#ff6b6b".to_string()),
            }],
        };
        charts.push(ChartData::Scatter(velocity_scatter));

        // 3. Fix time estimates as bar chart
        if !trends.fix_time_estimates.is_empty() {
            let mut categories = Vec::new();
            let mut values = Vec::new();

            for (category, duration) in &trends.fix_time_estimates {
                categories.push(format!("{category:?}"));
                values.push(duration.as_secs() as f64 / 60.0); // Convert to minutes
            }

            let fix_time_chart = BarChart {
                title: "Estimated Fix Times".to_string(),
                x_label: "Category".to_string(),
                y_label: "Minutes".to_string(),
                categories,
                values,
                colors: None,
            };
            charts.push(ChartData::Bar(fix_time_chart));
        }

        Ok(VisualizationData {
            charts,
            metadata: VisualizationMetadata {
                generated_at: SystemTime::now(),
                title: "Trend Analysis Dashboard".to_string(),
                description: "Comprehensive diagnostic trend analysis".to_string(),
                time_range: TimeRange {
                    start: SystemTime::now(),
                    end: SystemTime::now(),
                },
            },
        })
    }

    /// Export data in a format compatible with popular visualization libraries
    pub fn export_for_library(
        data: &VisualizationData,
        library: VisualizationLibrary,
    ) -> Result<String> {
        match library {
            VisualizationLibrary::Plotly => Self::to_plotly_format(data),
            VisualizationLibrary::ChartJs => Self::to_chartjs_format(data),
            VisualizationLibrary::D3 => Self::to_d3_format(data),
            VisualizationLibrary::Vega => Self::to_vega_format(data),
        }
    }

    // Private conversion methods

    fn to_plotly_format(data: &VisualizationData) -> Result<String> {
        let mut plotly_data = serde_json::json!({
            "data": [],
            "layout": {
                "title": data.metadata.title,
                "showlegend": true,
            }
        });

        for chart in &data.charts {
            match chart {
                ChartData::TimeSeries(ts) => {
                    for series in &ts.series {
                        let trace = serde_json::json!({
                            "type": "scatter",
                            "mode": "lines",
                            "name": series.name,
                            "x": series.data.iter().map(|p| p.x).collect::<Vec<_>>(),
                            "y": series.data.iter().map(|p| p.y).collect::<Vec<_>>(),
                            "line": {
                                "color": series.color,
                            }
                        });
                        plotly_data["data"].as_array_mut().unwrap().push(trace);
                    }
                }
                ChartData::Bar(bar) => {
                    let trace = serde_json::json!({
                        "type": "bar",
                        "x": bar.categories,
                        "y": bar.values,
                        "marker": {
                            "color": bar.colors,
                        }
                    });
                    plotly_data["data"].as_array_mut().unwrap().push(trace);
                }
                _ => {} // Add more chart types as needed
            }
        }

        Ok(serde_json::to_string_pretty(&plotly_data)?)
    }

    fn to_chartjs_format(data: &VisualizationData) -> Result<String> {
        let mut chartjs_configs = Vec::new();

        for chart in &data.charts {
            match chart {
                ChartData::TimeSeries(ts) => {
                    let config = serde_json::json!({
                        "type": "line",
                        "data": {
                            "labels": ts.series[0].data.iter().map(|p| p.x).collect::<Vec<_>>(),
                            "datasets": ts.series.iter().map(|s| {
                                serde_json::json!({
                                    "label": s.name,
                                    "data": s.data.iter().map(|p| p.y).collect::<Vec<_>>(),
                                    "borderColor": s.color,
                                    "fill": false,
                                })
                            }).collect::<Vec<_>>(),
                        },
                        "options": {
                            "responsive": true,
                            "title": {
                                "display": true,
                                "text": ts.title,
                            },
                        }
                    });
                    chartjs_configs.push(config);
                }
                _ => {} // Add more chart types as needed
            }
        }

        Ok(serde_json::to_string_pretty(&chartjs_configs)?)
    }

    fn to_d3_format(data: &VisualizationData) -> Result<String> {
        // D3 typically uses raw data, so we'll just export a clean data structure
        let d3_data = serde_json::json!({
            "metadata": data.metadata,
            "charts": data.charts,
        });

        Ok(serde_json::to_string_pretty(&d3_data)?)
    }

    fn to_vega_format(data: &VisualizationData) -> Result<String> {
        let mut vega_specs = Vec::new();

        for chart in &data.charts {
            match chart {
                ChartData::TimeSeries(ts) => {
                    let spec = serde_json::json!({
                        "$schema": "https://vega.github.io/schema/vega-lite/v5.json",
                        "title": ts.title,
                        "data": {
                            "values": ts.series.iter().flat_map(|s| {
                                s.data.iter().map(|p| {
                                    serde_json::json!({
                                        "x": p.x,
                                        "y": p.y,
                                        "series": s.name,
                                    })
                                })
                            }).collect::<Vec<_>>(),
                        },
                        "mark": "line",
                        "encoding": {
                            "x": {
                                "field": "x",
                                "type": "temporal",
                                "title": ts.x_label,
                            },
                            "y": {
                                "field": "y",
                                "type": "quantitative",
                                "title": ts.y_label,
                            },
                            "color": {
                                "field": "series",
                                "type": "nominal",
                            }
                        }
                    });
                    vega_specs.push(spec);
                }
                _ => {} // Add more chart types as needed
            }
        }

        Ok(serde_json::to_string_pretty(&vega_specs)?)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum VisualizationLibrary {
    Plotly,
    ChartJs,
    D3,
    Vega,
}

/// Generate HTML dashboard with embedded visualizations
pub fn generate_html_dashboard(data: &VisualizationData) -> Result<String> {
    let plotly_data =
        VisualizationExporter::export_for_library(data, VisualizationLibrary::Plotly)?;

    let html = format!(
        r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{}</title>
    <script src="https://cdn.plot.ly/plotly-latest.min.js"></script>
    <style>
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            margin: 0;
            padding: 20px;
            background-color: #f5f5f5;
        }}
        .header {{
            background-color: white;
            padding: 20px;
            border-radius: 8px;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
            margin-bottom: 20px;
        }}
        h1 {{
            margin: 0;
            color: #333;
        }}
        .description {{
            color: #666;
            margin-top: 10px;
        }}
        .chart-container {{
            background-color: white;
            padding: 20px;
            border-radius: 8px;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
            margin-bottom: 20px;
        }}
        .metadata {{
            font-size: 0.9em;
            color: #999;
            text-align: right;
            margin-top: 20px;
        }}
    </style>
</head>
<body>
    <div class="header">
        <h1>{}</h1>
        <div class="description">{}</div>
    </div>
    
    <div id="charts"></div>
    
    <div class="metadata">
        Generated at: {}
    </div>
    
    <script>
        const plotlyConfig = {};
        const chartsDiv = document.getElementById('charts');
        
        plotlyConfig.data.forEach((trace, index) => {{
            const div = document.createElement('div');
            div.className = 'chart-container';
            div.id = 'chart' + index;
            chartsDiv.appendChild(div);
            
            Plotly.newPlot(div.id, [trace], plotlyConfig.layout || {{}});
        }});
    </script>
</body>
</html>
"#,
        data.metadata.title,
        data.metadata.title,
        data.metadata.description,
        chrono::DateTime::<chrono::Utc>::from(data.metadata.generated_at).to_rfc3339(),
        plotly_data
    );

    Ok(html)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::UNIX_EPOCH;

    #[test]
    fn test_time_series_export() {
        let points = vec![TimeSeriesPoint {
            timestamp: UNIX_EPOCH + std::time::Duration::from_secs(1000),
            snapshot_count: 1,
            total_errors: 5,
            total_warnings: 10,
            avg_errors: 5.0,
            avg_warnings: 10.0,
            unique_files: 2,
        }];

        let result = VisualizationExporter::export_time_series(&points, "Test Chart").unwrap();
        assert_eq!(result.charts.len(), 1);

        if let ChartData::TimeSeries(ts) = &result.charts[0] {
            assert_eq!(ts.series.len(), 2);
            assert_eq!(ts.series[0].name, "Errors");
            assert_eq!(ts.series[1].name, "Warnings");
        } else {
            panic!("Expected TimeSeries chart");
        }
    }
}
