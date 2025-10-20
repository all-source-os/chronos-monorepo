use crate::error::{AllSourceError, Result};
use crate::event::Event;
use crate::store::EventStore;
use chrono::{DateTime, Datelike, Duration, Timelike, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Time window granularity for analytics
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TimeWindow {
    Minute,
    Hour,
    Day,
    Week,
    Month,
}

impl TimeWindow {
    pub fn duration(&self) -> Duration {
        match self {
            TimeWindow::Minute => Duration::minutes(1),
            TimeWindow::Hour => Duration::hours(1),
            TimeWindow::Day => Duration::days(1),
            TimeWindow::Week => Duration::weeks(1),
            TimeWindow::Month => Duration::days(30),
        }
    }

    pub fn truncate(&self, timestamp: DateTime<Utc>) -> DateTime<Utc> {
        match self {
            TimeWindow::Minute => timestamp
                .with_second(0)
                .unwrap()
                .with_nanosecond(0)
                .unwrap(),
            TimeWindow::Hour => timestamp
                .with_minute(0)
                .unwrap()
                .with_second(0)
                .unwrap()
                .with_nanosecond(0)
                .unwrap(),
            TimeWindow::Day => timestamp
                .with_hour(0)
                .unwrap()
                .with_minute(0)
                .unwrap()
                .with_second(0)
                .unwrap()
                .with_nanosecond(0)
                .unwrap(),
            TimeWindow::Week => {
                let days_from_monday = timestamp.weekday().num_days_from_monday();
                (timestamp - Duration::days(days_from_monday as i64))
                    .with_hour(0)
                    .unwrap()
                    .with_minute(0)
                    .unwrap()
                    .with_second(0)
                    .unwrap()
                    .with_nanosecond(0)
                    .unwrap()
            }
            TimeWindow::Month => timestamp
                .with_day(1)
                .unwrap()
                .with_hour(0)
                .unwrap()
                .with_minute(0)
                .unwrap()
                .with_second(0)
                .unwrap()
                .with_nanosecond(0)
                .unwrap(),
        }
    }
}

/// Request for event frequency analysis
#[derive(Debug, Deserialize)]
pub struct EventFrequencyRequest {
    /// Filter by entity ID
    pub entity_id: Option<String>,

    /// Filter by event type
    pub event_type: Option<String>,

    /// Start time for analysis
    pub since: DateTime<Utc>,

    /// End time for analysis (defaults to now)
    pub until: Option<DateTime<Utc>>,

    /// Time window granularity
    pub window: TimeWindow,
}

/// Time bucket with event count
#[derive(Debug, Clone, Serialize)]
pub struct TimeBucket {
    pub timestamp: DateTime<Utc>,
    pub count: usize,
    pub event_types: HashMap<String, usize>,
}

/// Response containing time-series frequency data
#[derive(Debug, Serialize)]
pub struct EventFrequencyResponse {
    pub buckets: Vec<TimeBucket>,
    pub total_events: usize,
    pub window: TimeWindow,
    pub time_range: TimeRange,
}

#[derive(Debug, Serialize)]
pub struct TimeRange {
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
}

/// Request for statistical summary
#[derive(Debug, Deserialize)]
pub struct StatsSummaryRequest {
    /// Filter by entity ID
    pub entity_id: Option<String>,

    /// Filter by event type
    pub event_type: Option<String>,

    /// Start time for analysis
    pub since: Option<DateTime<Utc>>,

    /// End time for analysis
    pub until: Option<DateTime<Utc>>,
}

/// Statistical summary response
#[derive(Debug, Serialize)]
pub struct StatsSummaryResponse {
    pub total_events: usize,
    pub unique_entities: usize,
    pub unique_event_types: usize,
    pub time_range: TimeRange,
    pub events_per_day: f64,
    pub top_event_types: Vec<EventTypeCount>,
    pub top_entities: Vec<EntityCount>,
    pub first_event: Option<DateTime<Utc>>,
    pub last_event: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct EventTypeCount {
    pub event_type: String,
    pub count: usize,
    pub percentage: f64,
}

#[derive(Debug, Serialize)]
pub struct EntityCount {
    pub entity_id: String,
    pub count: usize,
    pub percentage: f64,
}

/// Request for event correlation analysis
#[derive(Debug, Deserialize)]
pub struct CorrelationRequest {
    /// First event type
    pub event_type_a: String,

    /// Second event type
    pub event_type_b: String,

    /// Maximum time window to consider events correlated
    pub time_window_seconds: i64,

    /// Start time for analysis
    pub since: Option<DateTime<Utc>>,

    /// End time for analysis
    pub until: Option<DateTime<Utc>>,
}

/// Correlation analysis response
#[derive(Debug, Serialize)]
pub struct CorrelationResponse {
    pub event_type_a: String,
    pub event_type_b: String,
    pub total_a: usize,
    pub total_b: usize,
    pub correlated_pairs: usize,
    pub correlation_percentage: f64,
    pub avg_time_between_seconds: f64,
    pub examples: Vec<CorrelationExample>,
}

#[derive(Debug, Serialize)]
pub struct CorrelationExample {
    pub entity_id: String,
    pub event_a_timestamp: DateTime<Utc>,
    pub event_b_timestamp: DateTime<Utc>,
    pub time_between_seconds: i64,
}

/// Analytics engine for time-series and statistical analysis
pub struct AnalyticsEngine;

impl AnalyticsEngine {
    /// Analyze event frequency over time windows
    pub fn event_frequency(
        store: &EventStore,
        request: EventFrequencyRequest,
    ) -> Result<EventFrequencyResponse> {
        let until = request.until.unwrap_or_else(Utc::now);

        // Query events in the time range
        let events = store.query(crate::event::QueryEventsRequest {
            entity_id: request.entity_id.clone(),
            event_type: request.event_type.clone(),
            as_of: None,
            since: Some(request.since),
            until: Some(until),
            limit: None,
        })?;

        if events.is_empty() {
            return Ok(EventFrequencyResponse {
                buckets: Vec::new(),
                total_events: 0,
                window: request.window,
                time_range: TimeRange {
                    from: request.since,
                    to: until,
                },
            });
        }

        // Create time buckets
        let mut buckets_map: HashMap<DateTime<Utc>, HashMap<String, usize>> = HashMap::new();

        for event in &events {
            let bucket_time = request.window.truncate(event.timestamp);
            let bucket = buckets_map.entry(bucket_time).or_insert_with(HashMap::new);
            *bucket.entry(event.event_type.clone()).or_insert(0) += 1;
        }

        // Convert to sorted vector
        let mut buckets: Vec<TimeBucket> = buckets_map
            .into_iter()
            .map(|(timestamp, event_types)| {
                let count = event_types.values().sum();
                TimeBucket {
                    timestamp,
                    count,
                    event_types,
                }
            })
            .collect();

        buckets.sort_by_key(|b| b.timestamp);

        // Fill gaps in the timeline
        let filled_buckets = Self::fill_time_gaps(&buckets, request.since, until, request.window);

        Ok(EventFrequencyResponse {
            total_events: events.len(),
            buckets: filled_buckets,
            window: request.window,
            time_range: TimeRange {
                from: request.since,
                to: until,
            },
        })
    }

    /// Fill gaps in time buckets for continuous timeline
    fn fill_time_gaps(
        buckets: &[TimeBucket],
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        window: TimeWindow,
    ) -> Vec<TimeBucket> {
        if buckets.is_empty() {
            return Vec::new();
        }

        let mut filled = Vec::new();
        let mut current = window.truncate(start);
        let end = window.truncate(end);

        let bucket_map: HashMap<DateTime<Utc>, &TimeBucket> =
            buckets.iter().map(|b| (b.timestamp, b)).collect();

        while current <= end {
            if let Some(bucket) = bucket_map.get(&current) {
                filled.push((**bucket).clone());
            } else {
                filled.push(TimeBucket {
                    timestamp: current,
                    count: 0,
                    event_types: HashMap::new(),
                });
            }
            current = current + window.duration();
        }

        filled
    }

    /// Generate comprehensive statistical summary
    pub fn stats_summary(
        store: &EventStore,
        request: StatsSummaryRequest,
    ) -> Result<StatsSummaryResponse> {
        // Query events based on filters
        let events = store.query(crate::event::QueryEventsRequest {
            entity_id: request.entity_id.clone(),
            event_type: request.event_type.clone(),
            as_of: None,
            since: request.since,
            until: request.until,
            limit: None,
        })?;

        if events.is_empty() {
            return Err(AllSourceError::ValidationError(
                "No events found for the specified criteria".to_string(),
            ));
        }

        // Calculate statistics
        let first_event = events.first().map(|e| e.timestamp);
        let last_event = events.last().map(|e| e.timestamp);

        let mut entity_counts: HashMap<String, usize> = HashMap::new();
        let mut event_type_counts: HashMap<String, usize> = HashMap::new();

        for event in &events {
            *entity_counts.entry(event.entity_id.clone()).or_insert(0) += 1;
            *event_type_counts.entry(event.event_type.clone()).or_insert(0) += 1;
        }

        // Calculate events per day
        let time_span = if let (Some(first), Some(last)) = (first_event, last_event) {
            (last - first).num_days().max(1) as f64
        } else {
            1.0
        };

        let events_per_day = events.len() as f64 / time_span;

        // Top event types
        let mut top_event_types: Vec<EventTypeCount> = event_type_counts
            .into_iter()
            .map(|(event_type, count)| EventTypeCount {
                event_type,
                count,
                percentage: (count as f64 / events.len() as f64) * 100.0,
            })
            .collect();
        top_event_types.sort_by(|a, b| b.count.cmp(&a.count));
        top_event_types.truncate(10);

        // Top entities
        let mut top_entities: Vec<EntityCount> = entity_counts
            .into_iter()
            .map(|(entity_id, count)| EntityCount {
                entity_id,
                count,
                percentage: (count as f64 / events.len() as f64) * 100.0,
            })
            .collect();
        top_entities.sort_by(|a, b| b.count.cmp(&a.count));
        top_entities.truncate(10);

        let time_range = TimeRange {
            from: first_event.unwrap_or_else(Utc::now),
            to: last_event.unwrap_or_else(Utc::now),
        };

        Ok(StatsSummaryResponse {
            total_events: events.len(),
            unique_entities: top_entities.len(),
            unique_event_types: top_event_types.len(),
            time_range,
            events_per_day,
            top_event_types,
            top_entities,
            first_event,
            last_event,
        })
    }

    /// Analyze correlation between two event types
    pub fn analyze_correlation(
        store: &EventStore,
        request: CorrelationRequest,
    ) -> Result<CorrelationResponse> {
        // Query both event types
        let events_a = store.query(crate::event::QueryEventsRequest {
            entity_id: None,
            event_type: Some(request.event_type_a.clone()),
            as_of: None,
            since: request.since,
            until: request.until,
            limit: None,
        })?;

        let events_b = store.query(crate::event::QueryEventsRequest {
            entity_id: None,
            event_type: Some(request.event_type_b.clone()),
            as_of: None,
            since: request.since,
            until: request.until,
            limit: None,
        })?;

        // Group events by entity
        let mut entity_events_a: HashMap<String, Vec<&Event>> = HashMap::new();
        let mut entity_events_b: HashMap<String, Vec<&Event>> = HashMap::new();

        for event in &events_a {
            entity_events_a
                .entry(event.entity_id.clone())
                .or_insert_with(Vec::new)
                .push(event);
        }

        for event in &events_b {
            entity_events_b
                .entry(event.entity_id.clone())
                .or_insert_with(Vec::new)
                .push(event);
        }

        // Find correlated pairs
        let mut correlated_pairs = 0;
        let mut total_time_between = 0i64;
        let mut examples = Vec::new();

        for (entity_id, a_events) in &entity_events_a {
            if let Some(b_events) = entity_events_b.get(entity_id) {
                for a_event in a_events {
                    for b_event in b_events {
                        let time_diff = (b_event.timestamp - a_event.timestamp).num_seconds().abs();

                        if time_diff <= request.time_window_seconds {
                            correlated_pairs += 1;
                            total_time_between += time_diff;

                            if examples.len() < 5 {
                                examples.push(CorrelationExample {
                                    entity_id: entity_id.clone(),
                                    event_a_timestamp: a_event.timestamp,
                                    event_b_timestamp: b_event.timestamp,
                                    time_between_seconds: time_diff,
                                });
                            }
                        }
                    }
                }
            }
        }

        let correlation_percentage = if !events_a.is_empty() {
            (correlated_pairs as f64 / events_a.len() as f64) * 100.0
        } else {
            0.0
        };

        let avg_time_between = if correlated_pairs > 0 {
            total_time_between as f64 / correlated_pairs as f64
        } else {
            0.0
        };

        Ok(CorrelationResponse {
            event_type_a: request.event_type_a,
            event_type_b: request.event_type_b,
            total_a: events_a.len(),
            total_b: events_b.len(),
            correlated_pairs,
            correlation_percentage,
            avg_time_between_seconds: avg_time_between,
            examples,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_window_truncation() {
        let timestamp = chrono::Utc::now();

        let minute_truncated = TimeWindow::Minute.truncate(timestamp);
        assert_eq!(minute_truncated.second(), 0);

        let hour_truncated = TimeWindow::Hour.truncate(timestamp);
        assert_eq!(hour_truncated.minute(), 0);
        assert_eq!(hour_truncated.second(), 0);

        let day_truncated = TimeWindow::Day.truncate(timestamp);
        assert_eq!(day_truncated.hour(), 0);
        assert_eq!(day_truncated.minute(), 0);
    }
}
