// Advanced query features (v2.0)
//
// - eventql: SQL-like query language using DataFusion over Arrow RecordBatches
// - graphql: Lightweight GraphQL API for events and projections
// - geospatial: Haversine distance and bounding box queries on event coordinates

#[cfg(feature = "analytics")]
pub mod eventql;
pub mod geospatial;
pub mod graphql;
