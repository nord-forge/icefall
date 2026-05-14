use axum::extract::{Path, State};
use axum::Json;

use crate::api::error::ApiError;
use crate::api::AppState;

pub async fn server_forecast(
    State(state): State<AppState>,
    Path(server_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let data = state
        .db
        .get_server_metrics_for_forecast(&server_id, 30)
        .await?;

    if data.len() < 7 {
        return Ok(Json(serde_json::json!({
            "data": {
                "message": "Not enough data for forecast (need at least 7 days)",
                "days_available": data.len(),
            }
        })));
    }

    let disk_forecast = linear_forecast(&data.iter().map(|(d, _, _)| *d).collect::<Vec<_>>());
    let mem_forecast = linear_forecast(&data.iter().map(|(_, m, _)| *m).collect::<Vec<_>>());
    let cpu_forecast = linear_forecast(&data.iter().map(|(_, _, c)| *c).collect::<Vec<_>>());

    let disk_days_until_full = if disk_forecast.slope > 0.0 {
        Some(((1.0 - disk_forecast.last_value) / disk_forecast.slope).max(0.0) as i64)
    } else {
        None
    };

    let mem_days_until_full = if mem_forecast.slope > 0.0 {
        Some(((1.0 - mem_forecast.last_value) / mem_forecast.slope).max(0.0) as i64)
    } else {
        None
    };

    Ok(Json(serde_json::json!({
        "data": {
            "disk": {
                "current_ratio": disk_forecast.last_value,
                "daily_growth": disk_forecast.slope,
                "days_until_full": disk_days_until_full,
            },
            "memory": {
                "current_ratio": mem_forecast.last_value,
                "daily_growth": mem_forecast.slope,
                "days_until_full": mem_days_until_full,
            },
            "cpu": {
                "current_percent": cpu_forecast.last_value,
                "daily_trend": cpu_forecast.slope,
            },
            "data_points": data.len(),
        }
    })))
}

struct ForecastResult {
    slope: f64,
    last_value: f64,
}

fn linear_forecast(values: &[f64]) -> ForecastResult {
    let n = values.len() as f64;
    if n < 2.0 {
        return ForecastResult {
            slope: 0.0,
            last_value: values.first().copied().unwrap_or(0.0),
        };
    }

    let x_mean = (n - 1.0) / 2.0;
    let y_mean: f64 = values.iter().sum::<f64>() / n;

    let mut num = 0.0;
    let mut den = 0.0;
    for (i, y) in values.iter().enumerate() {
        let x = i as f64;
        num += (x - x_mean) * (y - y_mean);
        den += (x - x_mean) * (x - x_mean);
    }

    let slope = if den > 0.0 { num / den } else { 0.0 };
    let last_value = *values.last().unwrap_or(&0.0);

    ForecastResult { slope, last_value }
}
