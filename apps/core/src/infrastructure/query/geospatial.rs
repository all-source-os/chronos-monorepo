//! Geospatial query support for AllSource events
//!
//! Enables spatial filtering of events based on coordinates stored in event
//! metadata or payload. Supports:
//!
//! - **Bounding box queries**: Find events within a rectangular region
//! - **Radius queries**: Find events within a distance of a point (haversine)
//! - **Geospatial indexing**: In-memory spatial index using grid cells for O(1) lookups
//!
//! ## Coordinate extraction
//!
//! Coordinates are extracted from events by checking (in order):
//! 1. `metadata.location.lat` / `metadata.location.lng`
//! 2. `metadata.lat` / `metadata.lng`
//! 3. `payload.location.lat` / `payload.location.lng`
//! 4. `payload.lat` / `payload.lng`
//! 5. `metadata.latitude` / `metadata.longitude`
//! 6. `payload.latitude` / `payload.longitude`
//!
//! ## Limitations
//!
//! - Grid-based spatial index (not R-tree) — adequate for moderate datasets
//! - Haversine assumes spherical Earth (max ~0.3% error vs ellipsoidal)
//! - Events without coordinates are silently excluded from spatial queries

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::entities::Event;

/// Earth's radius in kilometers
const EARTH_RADIUS_KM: f64 = 6371.0;

/// Grid cell size in degrees (approximately 11km at equator)
const GRID_CELL_SIZE: f64 = 0.1;

/// A geographic coordinate
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Coordinate {
    pub lat: f64,
    pub lng: f64,
}

impl Coordinate {
    pub fn new(lat: f64, lng: f64) -> Result<Self, String> {
        if !(-90.0..=90.0).contains(&lat) {
            return Err(format!("Latitude {lat} out of range [-90, 90]"));
        }
        if !(-180.0..=180.0).contains(&lng) {
            return Err(format!("Longitude {lng} out of range [-180, 180]"));
        }
        Ok(Self { lat, lng })
    }
}

/// Bounding box for rectangular queries
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct BoundingBox {
    pub min_lat: f64,
    pub min_lng: f64,
    pub max_lat: f64,
    pub max_lng: f64,
}

impl BoundingBox {
    pub fn contains(&self, coord: &Coordinate) -> bool {
        coord.lat >= self.min_lat
            && coord.lat <= self.max_lat
            && coord.lng >= self.min_lng
            && coord.lng <= self.max_lng
    }
}

/// Radius query parameters
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct RadiusQuery {
    pub center_lat: f64,
    pub center_lng: f64,
    /// Radius in kilometers
    pub radius_km: f64,
}

/// Geospatial query request
#[derive(Debug, Clone, Deserialize)]
pub struct GeoQueryRequest {
    /// Bounding box filter (optional)
    pub bbox: Option<BoundingBox>,
    /// Radius filter (optional, applied after bbox if both present)
    pub radius: Option<RadiusQuery>,
    /// Additional event_type filter
    pub event_type: Option<String>,
    /// Additional tenant_id filter
    pub tenant_id: Option<String>,
    /// Maximum results
    pub limit: Option<usize>,
    /// Sort by distance from a point (only meaningful with radius query)
    pub sort_by_distance: Option<bool>,
}

/// A geo-enriched event result
#[derive(Debug, Clone, Serialize)]
pub struct GeoEventResult {
    pub event: serde_json::Value,
    pub coordinate: Coordinate,
    /// Distance from query center in km (only set for radius queries)
    pub distance_km: Option<f64>,
}

/// Grid cell key for the spatial index
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
struct GridCell {
    lat_cell: i32,
    lng_cell: i32,
}

impl GridCell {
    fn from_coordinate(coord: &Coordinate) -> Self {
        Self {
            lat_cell: (coord.lat / GRID_CELL_SIZE).floor() as i32,
            lng_cell: (coord.lng / GRID_CELL_SIZE).floor() as i32,
        }
    }
}

/// In-memory geospatial index using grid cells
pub struct GeoIndex {
    /// Grid cell → list of (event_id, coordinate) entries
    grid: DashMap<(i32, i32), Vec<(Uuid, Coordinate)>>,
    /// Event ID → coordinate for quick lookups
    coordinates: DashMap<Uuid, Coordinate>,
}

impl Default for GeoIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl GeoIndex {
    pub fn new() -> Self {
        Self {
            grid: DashMap::new(),
            coordinates: DashMap::new(),
        }
    }

    /// Index an event's coordinates (if extractable)
    pub fn index_event(&self, event: &Event) {
        if let Some(coord) = extract_coordinates(event) {
            let cell = GridCell::from_coordinate(&coord);
            self.grid
                .entry((cell.lat_cell, cell.lng_cell))
                .or_default()
                .push((event.id, coord));
            self.coordinates.insert(event.id, coord);
        }
    }

    /// Get coordinate for an event
    pub fn get_coordinate(&self, event_id: &Uuid) -> Option<Coordinate> {
        self.coordinates.get(event_id).map(|c| *c)
    }

    /// Find all event IDs within a bounding box
    pub fn query_bbox(&self, bbox: &BoundingBox) -> Vec<(Uuid, Coordinate)> {
        let min_cell = GridCell::from_coordinate(&Coordinate {
            lat: bbox.min_lat,
            lng: bbox.min_lng,
        });
        let max_cell = GridCell::from_coordinate(&Coordinate {
            lat: bbox.max_lat,
            lng: bbox.max_lng,
        });

        let mut results = Vec::new();
        for lat_cell in min_cell.lat_cell..=max_cell.lat_cell {
            for lng_cell in min_cell.lng_cell..=max_cell.lng_cell {
                if let Some(entries) = self.grid.get(&(lat_cell, lng_cell)) {
                    for (id, coord) in entries.iter() {
                        if bbox.contains(coord) {
                            results.push((*id, *coord));
                        }
                    }
                }
            }
        }
        results
    }

    /// Find all event IDs within a radius
    pub fn query_radius(&self, query: &RadiusQuery) -> Vec<(Uuid, Coordinate, f64)> {
        // Compute bounding box for the radius to narrow grid cells
        let lat_delta = query.radius_km / 111.0; // ~111 km per degree latitude
        let lng_delta = query.radius_km / (111.0 * query.center_lat.to_radians().cos().max(0.001));
        let bbox = BoundingBox {
            min_lat: query.center_lat - lat_delta,
            max_lat: query.center_lat + lat_delta,
            min_lng: query.center_lng - lng_delta,
            max_lng: query.center_lng + lng_delta,
        };

        let candidates = self.query_bbox(&bbox);
        let center = Coordinate {
            lat: query.center_lat,
            lng: query.center_lng,
        };

        candidates
            .into_iter()
            .filter_map(|(id, coord)| {
                let dist = haversine_distance(&center, &coord);
                if dist <= query.radius_km {
                    Some((id, coord, dist))
                } else {
                    None
                }
            })
            .collect()
    }

    /// Number of indexed events
    pub fn len(&self) -> usize {
        self.coordinates.len()
    }

    pub fn is_empty(&self) -> bool {
        self.coordinates.is_empty()
    }

    /// Statistics about the geo index
    pub fn stats(&self) -> GeoIndexStats {
        GeoIndexStats {
            indexed_events: self.coordinates.len(),
            grid_cells: self.grid.len(),
        }
    }
}

/// Geo index statistics
#[derive(Debug, Clone, Serialize)]
pub struct GeoIndexStats {
    pub indexed_events: usize,
    pub grid_cells: usize,
}

/// Haversine distance between two coordinates in kilometers
pub fn haversine_distance(a: &Coordinate, b: &Coordinate) -> f64 {
    let d_lat = (b.lat - a.lat).to_radians();
    let d_lng = (b.lng - a.lng).to_radians();
    let lat1 = a.lat.to_radians();
    let lat2 = b.lat.to_radians();

    let h = (d_lat / 2.0).sin().powi(2) + lat1.cos() * lat2.cos() * (d_lng / 2.0).sin().powi(2);
    let c = 2.0 * h.sqrt().asin();

    EARTH_RADIUS_KM * c
}

/// Extract coordinates from an event's metadata or payload
pub fn extract_coordinates(event: &Event) -> Option<Coordinate> {
    // Try metadata first
    if let Some(ref meta) = event.metadata
        && let Some(coord) = try_extract_coord(meta)
    {
        return Some(coord);
    }
    // Fall back to payload
    try_extract_coord(&event.payload)
}

/// Try to extract coordinates from a JSON value
fn try_extract_coord(value: &serde_json::Value) -> Option<Coordinate> {
    // Try nested location object
    if let Some(loc) = value.get("location") {
        if let (Some(lat), Some(lng)) = (
            loc.get("lat").and_then(|v| v.as_f64()),
            loc.get("lng").and_then(|v| v.as_f64()),
        ) && let Ok(c) = Coordinate::new(lat, lng)
        {
            return Some(c);
        }
        if let (Some(lat), Some(lng)) = (
            loc.get("latitude").and_then(|v| v.as_f64()),
            loc.get("longitude").and_then(|v| v.as_f64()),
        ) && let Ok(c) = Coordinate::new(lat, lng)
        {
            return Some(c);
        }
    }

    // Try flat lat/lng
    if let (Some(lat), Some(lng)) = (
        value.get("lat").and_then(|v| v.as_f64()),
        value.get("lng").and_then(|v| v.as_f64()),
    ) && let Ok(c) = Coordinate::new(lat, lng)
    {
        return Some(c);
    }

    // Try flat latitude/longitude
    if let (Some(lat), Some(lng)) = (
        value.get("latitude").and_then(|v| v.as_f64()),
        value.get("longitude").and_then(|v| v.as_f64()),
    ) && let Ok(c) = Coordinate::new(lat, lng)
    {
        return Some(c);
    }

    None
}

/// Execute a geospatial query against events using the geo index
pub fn execute_geo_query(
    events: &[Event],
    geo_index: &GeoIndex,
    request: &GeoQueryRequest,
) -> Vec<GeoEventResult> {
    // Determine candidate event IDs from spatial filters
    let candidates: Vec<(Uuid, Coordinate, Option<f64>)> = if let Some(ref radius) = request.radius
    {
        geo_index
            .query_radius(radius)
            .into_iter()
            .map(|(id, coord, dist)| (id, coord, Some(dist)))
            .collect()
    } else if let Some(ref bbox) = request.bbox {
        geo_index
            .query_bbox(bbox)
            .into_iter()
            .map(|(id, coord)| (id, coord, None))
            .collect()
    } else {
        // No spatial filter — return all geo-indexed events
        geo_index
            .coordinates
            .iter()
            .map(|entry| (*entry.key(), *entry.value(), None))
            .collect()
    };

    // Build a set of candidate IDs for fast lookup
    let candidate_map: std::collections::HashMap<Uuid, (Coordinate, Option<f64>)> = candidates
        .into_iter()
        .map(|(id, coord, dist)| (id, (coord, dist)))
        .collect();

    // Filter events by candidates + additional filters
    let mut results: Vec<GeoEventResult> = events
        .iter()
        .filter_map(|event| {
            let (coord, dist) = candidate_map.get(&event.id)?;

            // Apply event_type filter
            if let Some(ref et) = request.event_type
                && event.event_type_str() != et
            {
                return None;
            }

            // Apply tenant_id filter
            if let Some(ref tid) = request.tenant_id
                && event.tenant_id_str() != tid
            {
                return None;
            }

            Some(GeoEventResult {
                event: serde_json::json!({
                    "id": event.id.to_string(),
                    "event_type": event.event_type_str(),
                    "entity_id": event.entity_id_str(),
                    "tenant_id": event.tenant_id_str(),
                    "payload": event.payload,
                    "metadata": event.metadata,
                    "timestamp": event.timestamp.to_rfc3339(),
                    "version": event.version,
                }),
                coordinate: *coord,
                distance_km: *dist,
            })
        })
        .collect();

    // Sort by distance if requested
    if request.sort_by_distance.unwrap_or(false) {
        results.sort_by(|a, b| {
            a.distance_km
                .unwrap_or(f64::MAX)
                .partial_cmp(&b.distance_km.unwrap_or(f64::MAX))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }

    // Apply limit
    if let Some(limit) = request.limit {
        results.truncate(limit);
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_geo_event(entity: &str, lat: f64, lng: f64) -> Event {
        Event::from_strings(
            "location.update".to_string(),
            entity.to_string(),
            "default".to_string(),
            serde_json::json!({"action": "checkin"}),
            Some(serde_json::json!({"location": {"lat": lat, "lng": lng}})),
        )
        .unwrap()
    }

    #[test]
    fn test_haversine_same_point() {
        let a = Coordinate { lat: 0.0, lng: 0.0 };
        assert!((haversine_distance(&a, &a) - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_haversine_known_distance() {
        // NYC to London is approximately 5570 km
        let nyc = Coordinate {
            lat: 40.7128,
            lng: -74.0060,
        };
        let london = Coordinate {
            lat: 51.5074,
            lng: -0.1278,
        };
        let dist = haversine_distance(&nyc, &london);
        assert!((dist - 5570.0).abs() < 50.0, "NYC-London: {dist} km");
    }

    #[test]
    fn test_coordinate_validation() {
        assert!(Coordinate::new(45.0, 90.0).is_ok());
        assert!(Coordinate::new(91.0, 0.0).is_err());
        assert!(Coordinate::new(0.0, 181.0).is_err());
    }

    #[test]
    fn test_bounding_box_contains() {
        let bbox = BoundingBox {
            min_lat: 40.0,
            max_lat: 42.0,
            min_lng: -75.0,
            max_lng: -73.0,
        };
        assert!(bbox.contains(&Coordinate {
            lat: 41.0,
            lng: -74.0
        }));
        assert!(!bbox.contains(&Coordinate {
            lat: 43.0,
            lng: -74.0
        }));
    }

    #[test]
    fn test_extract_coordinates_from_metadata_location() {
        let event = make_geo_event("e1", 40.7128, -74.0060);
        let coord = extract_coordinates(&event).unwrap();
        assert!((coord.lat - 40.7128).abs() < 0.001);
        assert!((coord.lng - (-74.0060)).abs() < 0.001);
    }

    #[test]
    fn test_extract_coordinates_from_payload_flat() {
        let event = Event::from_strings(
            "location.update".to_string(),
            "e1".to_string(),
            "default".to_string(),
            serde_json::json!({"lat": 51.5074, "lng": -0.1278}),
            None,
        )
        .unwrap();
        let coord = extract_coordinates(&event).unwrap();
        assert!((coord.lat - 51.5074).abs() < 0.001);
    }

    #[test]
    fn test_no_coordinates() {
        let event = Event::from_strings(
            "user.created".to_string(),
            "u1".to_string(),
            "default".to_string(),
            serde_json::json!({"name": "Alice"}),
            None,
        )
        .unwrap();
        assert!(extract_coordinates(&event).is_none());
    }

    #[test]
    fn test_geo_index_and_bbox_query() {
        let index = GeoIndex::new();
        let events = vec![
            make_geo_event("e1", 40.7128, -74.0060), // NYC
            make_geo_event("e2", 51.5074, -0.1278),  // London
            make_geo_event("e3", 41.0, -73.5),       // Near NYC
        ];
        for e in &events {
            index.index_event(e);
        }
        assert_eq!(index.len(), 3);

        let bbox = BoundingBox {
            min_lat: 40.0,
            max_lat: 42.0,
            min_lng: -75.0,
            max_lng: -73.0,
        };
        let results = index.query_bbox(&bbox);
        assert_eq!(results.len(), 2); // NYC and Near NYC
    }

    #[test]
    fn test_geo_index_radius_query() {
        let index = GeoIndex::new();
        let events = vec![
            make_geo_event("e1", 40.7128, -74.0060), // NYC
            make_geo_event("e2", 51.5074, -0.1278),  // London
            make_geo_event("e3", 40.75, -73.99),     // Near NYC (~5km)
        ];
        for e in &events {
            index.index_event(e);
        }

        let query = RadiusQuery {
            center_lat: 40.7128,
            center_lng: -74.0060,
            radius_km: 10.0,
        };
        let results = index.query_radius(&query);
        assert_eq!(results.len(), 2); // NYC and Near NYC
        for (_, _, dist) in &results {
            assert!(*dist <= 10.0);
        }
    }

    #[test]
    fn test_execute_geo_query_with_type_filter() {
        let index = GeoIndex::new();
        let mut events = vec![make_geo_event("e1", 40.7128, -74.0060)];
        let other = Event::from_strings(
            "user.created".to_string(),
            "e2".to_string(),
            "default".to_string(),
            serde_json::json!({"lat": 40.75, "lng": -73.99}),
            None,
        )
        .unwrap();
        events.push(other);
        for e in &events {
            index.index_event(e);
        }

        let req = GeoQueryRequest {
            bbox: Some(BoundingBox {
                min_lat: 40.0,
                max_lat: 42.0,
                min_lng: -75.0,
                max_lng: -73.0,
            }),
            radius: None,
            event_type: Some("location.update".to_string()),
            tenant_id: None,
            limit: None,
            sort_by_distance: None,
        };
        let results = execute_geo_query(&events, &index, &req);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_geo_index_stats() {
        let index = GeoIndex::new();
        let events = vec![
            make_geo_event("e1", 40.7128, -74.0060),
            make_geo_event("e2", 51.5074, -0.1278),
        ];
        for e in &events {
            index.index_event(e);
        }
        let stats = index.stats();
        assert_eq!(stats.indexed_events, 2);
        assert!(stats.grid_cells >= 1);
    }
}
