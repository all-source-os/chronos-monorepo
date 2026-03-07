use std::{fmt, str::FromStr};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::error::ChronError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Open,
    #[serde(rename = "in-progress")]
    InProgress,
    Done,
}

impl fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Open => write!(f, "open"),
            Self::InProgress => write!(f, "in-progress"),
            Self::Done => write!(f, "done"),
        }
    }
}

impl FromStr for TaskStatus {
    type Err = ChronError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "open" => Ok(Self::Open),
            "in-progress" | "in_progress" | "inprogress" => Ok(Self::InProgress),
            "done" => Ok(Self::Done),
            other => Err(ChronError::InvalidStatus(other.to_string())),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    P0,
    P1,
    P2,
    P3,
}

impl fmt::Display for Priority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::P0 => write!(f, "p0"),
            Self::P1 => write!(f, "p1"),
            Self::P2 => write!(f, "p2"),
            Self::P3 => write!(f, "p3"),
        }
    }
}

impl FromStr for Priority {
    type Err = ChronError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "p0" => Ok(Self::P0),
            "p1" => Ok(Self::P1),
            "p2" => Ok(Self::P2),
            "p3" => Ok(Self::P3),
            other => Err(ChronError::InvalidPriority(other.to_string())),
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskType {
    #[default]
    Task,
    Epic,
    Bug,
    Feature,
}

impl fmt::Display for TaskType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Task => write!(f, "task"),
            Self::Epic => write!(f, "epic"),
            Self::Bug => write!(f, "bug"),
            Self::Feature => write!(f, "feature"),
        }
    }
}

impl FromStr for TaskType {
    type Err = ChronError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "task" => Ok(Self::Task),
            "epic" => Ok(Self::Epic),
            "bug" => Ok(Self::Bug),
            "feature" => Ok(Self::Feature),
            other => Err(ChronError::InvalidTaskType(other.to_string())),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub priority: Priority,
    pub status: TaskStatus,
    #[serde(default)]
    pub task_type: TaskType,
    pub parent: Option<String>,
    pub claimed_by: Option<String>,
    pub blocked_by: Vec<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub done_reason: Option<String>,
    pub done_at: Option<DateTime<Utc>>,
    pub awaiting_approval: Option<bool>,
    pub approved: Option<bool>,
    pub approved_at: Option<DateTime<Utc>>,
    pub description: Option<String>,
    #[serde(default)]
    pub archived: bool,
}
