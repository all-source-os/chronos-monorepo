use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Value Object: Version
///
/// Represents a version number for entities like events, schemas, or projections.
///
/// Domain Rules:
/// - Must be a positive integer (>= 1)
/// - Immutable once created
/// - Sequential within a stream
///
/// This is a Value Object:
/// - Defined by its value, not identity
/// - Immutable
/// - Self-validating
/// - Compared by value equality
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Version(u64);

impl Version {
    /// The first version number (always 1)
    pub const FIRST: Version = Version(1);

    /// Create a new Version with validation
    ///
    /// # Errors
    /// Returns error if version is 0
    ///
    /// # Examples
    /// ```
    /// use allsource_core::domain::value_objects::Version;
    ///
    /// let version = Version::new(1).unwrap();
    /// assert_eq!(version.as_u64(), 1);
    /// ```
    pub fn new(value: u64) -> Result<Self> {
        Self::validate(value)?;
        Ok(Self(value))
    }

    /// Create Version without validation (for internal use)
    ///
    /// # Safety
    /// This bypasses validation. Only use when loading from trusted sources.
    pub(crate) fn new_unchecked(value: u64) -> Self {
        Self(value)
    }

    /// Create the first version
    ///
    /// # Examples
    /// ```
    /// use allsource_core::domain::value_objects::Version;
    ///
    /// let version = Version::first();
    /// assert_eq!(version.as_u64(), 1);
    /// assert!(version.is_first());
    /// ```
    pub fn first() -> Self {
        Self::FIRST
    }

    /// Get the version value as u64
    pub fn as_u64(&self) -> u64 {
        self.0
    }

    /// Get the version value as i64 (for compatibility)
    pub fn as_i64(&self) -> i64 {
        self.0 as i64
    }

    /// Get the version value as u32 (for compatibility)
    pub fn as_u32(&self) -> u32 {
        self.0 as u32
    }

    /// Check if this is the first version
    ///
    /// # Examples
    /// ```
    /// use allsource_core::domain::value_objects::Version;
    ///
    /// let v1 = Version::first();
    /// let v2 = Version::new(2).unwrap();
    ///
    /// assert!(v1.is_first());
    /// assert!(!v2.is_first());
    /// ```
    pub fn is_first(&self) -> bool {
        self.0 == 1
    }

    /// Get the next version
    ///
    /// # Examples
    /// ```
    /// use allsource_core::domain::value_objects::Version;
    ///
    /// let v1 = Version::first();
    /// let v2 = v1.next();
    ///
    /// assert_eq!(v2.as_u64(), 2);
    /// ```
    pub fn next(&self) -> Self {
        Self(self.0.saturating_add(1))
    }

    /// Get the previous version, if possible
    ///
    /// Returns None if this is the first version.
    ///
    /// # Examples
    /// ```
    /// use allsource_core::domain::value_objects::Version;
    ///
    /// let v2 = Version::new(2).unwrap();
    /// let v1 = v2.previous();
    ///
    /// assert_eq!(v1, Some(Version::first()));
    ///
    /// let first = Version::first();
    /// assert_eq!(first.previous(), None);
    /// ```
    pub fn previous(&self) -> Option<Self> {
        if self.0 > 1 {
            Some(Self(self.0 - 1))
        } else {
            None
        }
    }

    /// Check if this version is immediately after another
    ///
    /// # Examples
    /// ```
    /// use allsource_core::domain::value_objects::Version;
    ///
    /// let v1 = Version::first();
    /// let v2 = Version::new(2).unwrap();
    /// let v3 = Version::new(3).unwrap();
    ///
    /// assert!(v2.is_immediately_after(&v1));
    /// assert!(!v3.is_immediately_after(&v1));
    /// ```
    pub fn is_immediately_after(&self, other: &Version) -> bool {
        self.0 == other.0 + 1
    }

    /// Validate a version number
    fn validate(value: u64) -> Result<()> {
        if value == 0 {
            return Err(crate::error::AllSourceError::InvalidInput(
                "Version must be >= 1".to_string(),
            ));
        }
        Ok(())
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<u64> for Version {
    type Error = crate::error::AllSourceError;

    fn try_from(value: u64) -> Result<Self> {
        Version::new(value)
    }
}

impl TryFrom<i64> for Version {
    type Error = crate::error::AllSourceError;

    fn try_from(value: i64) -> Result<Self> {
        if value < 1 {
            return Err(crate::error::AllSourceError::InvalidInput(
                "Version must be >= 1".to_string(),
            ));
        }
        Version::new(value as u64)
    }
}

impl TryFrom<u32> for Version {
    type Error = crate::error::AllSourceError;

    fn try_from(value: u32) -> Result<Self> {
        Version::new(value as u64)
    }
}

impl From<Version> for u64 {
    fn from(version: Version) -> Self {
        version.0
    }
}

impl From<Version> for i64 {
    fn from(version: Version) -> Self {
        version.0 as i64
    }
}

impl From<Version> for u32 {
    fn from(version: Version) -> Self {
        version.0 as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_valid_version() {
        let version = Version::new(1);
        assert!(version.is_ok());
        assert_eq!(version.unwrap().as_u64(), 1);

        let version = Version::new(100);
        assert!(version.is_ok());
        assert_eq!(version.unwrap().as_u64(), 100);
    }

    #[test]
    fn test_reject_zero_version() {
        let result = Version::new(0);
        assert!(result.is_err());

        if let Err(e) = result {
            assert!(e.to_string().contains("must be >= 1"));
        }
    }

    #[test]
    fn test_first_version() {
        let version = Version::first();
        assert_eq!(version.as_u64(), 1);
        assert!(version.is_first());
    }

    #[test]
    fn test_is_first() {
        let v1 = Version::first();
        let v2 = Version::new(2).unwrap();

        assert!(v1.is_first());
        assert!(!v2.is_first());
    }

    #[test]
    fn test_next_version() {
        let v1 = Version::first();
        let v2 = v1.next();
        let v3 = v2.next();

        assert_eq!(v2.as_u64(), 2);
        assert_eq!(v3.as_u64(), 3);
    }

    #[test]
    fn test_previous_version() {
        let v1 = Version::first();
        let v2 = Version::new(2).unwrap();
        let v3 = Version::new(3).unwrap();

        assert_eq!(v1.previous(), None);
        assert_eq!(v2.previous(), Some(Version::first()));
        assert_eq!(v3.previous(), Some(Version::new(2).unwrap()));
    }

    #[test]
    fn test_is_immediately_after() {
        let v1 = Version::first();
        let v2 = Version::new(2).unwrap();
        let v3 = Version::new(3).unwrap();

        assert!(v2.is_immediately_after(&v1));
        assert!(v3.is_immediately_after(&v2));
        assert!(!v3.is_immediately_after(&v1));
    }

    #[test]
    fn test_version_ordering() {
        let v1 = Version::first();
        let v2 = Version::new(2).unwrap();
        let v3 = Version::new(3).unwrap();

        assert!(v1 < v2);
        assert!(v2 < v3);
        assert!(v1 < v3);
        assert!(v3 > v1);
    }

    #[test]
    fn test_as_conversions() {
        let version = Version::new(42).unwrap();

        assert_eq!(version.as_u64(), 42);
        assert_eq!(version.as_i64(), 42);
        assert_eq!(version.as_u32(), 42);
    }

    #[test]
    fn test_display_trait() {
        let version = Version::new(42).unwrap();
        assert_eq!(format!("{}", version), "42");
    }

    #[test]
    fn test_try_from_u64() {
        let version: Result<Version> = 5u64.try_into();
        assert!(version.is_ok());
        assert_eq!(version.unwrap().as_u64(), 5);

        let invalid: Result<Version> = 0u64.try_into();
        assert!(invalid.is_err());
    }

    #[test]
    fn test_try_from_i64() {
        let version: Result<Version> = 5i64.try_into();
        assert!(version.is_ok());
        assert_eq!(version.unwrap().as_u64(), 5);

        let invalid: Result<Version> = 0i64.try_into();
        assert!(invalid.is_err());

        let negative: Result<Version> = (-1i64).try_into();
        assert!(negative.is_err());
    }

    #[test]
    fn test_try_from_u32() {
        let version: Result<Version> = 5u32.try_into();
        assert!(version.is_ok());
        assert_eq!(version.unwrap().as_u64(), 5);

        let invalid: Result<Version> = 0u32.try_into();
        assert!(invalid.is_err());
    }

    #[test]
    fn test_into_conversions() {
        let version = Version::new(42).unwrap();

        let u64_val: u64 = version.into();
        assert_eq!(u64_val, 42);

        let version = Version::new(42).unwrap();
        let i64_val: i64 = version.into();
        assert_eq!(i64_val, 42);

        let version = Version::new(42).unwrap();
        let u32_val: u32 = version.into();
        assert_eq!(u32_val, 42);
    }

    #[test]
    fn test_equality() {
        let v1 = Version::new(1).unwrap();
        let v2 = Version::new(1).unwrap();
        let v3 = Version::new(2).unwrap();

        assert_eq!(v1, v2);
        assert_ne!(v1, v3);
    }

    #[test]
    fn test_cloning() {
        let v1 = Version::new(5).unwrap();
        let v2 = v1; // Copy
        assert_eq!(v1, v2);
    }

    #[test]
    fn test_hash_consistency() {
        use std::collections::HashSet;

        let v1 = Version::new(5).unwrap();
        let v2 = Version::new(5).unwrap();

        let mut set = HashSet::new();
        set.insert(v1);

        assert!(set.contains(&v2));
    }

    #[test]
    fn test_serde_serialization() {
        let version = Version::new(42).unwrap();

        // Serialize
        let json = serde_json::to_string(&version).unwrap();
        assert_eq!(json, "42");

        // Deserialize
        let deserialized: Version = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, version);
    }

    #[test]
    fn test_const_first() {
        assert_eq!(Version::FIRST.as_u64(), 1);
        assert_eq!(Version::FIRST, Version::first());
    }

    #[test]
    fn test_next_saturation() {
        let max_version = Version::new_unchecked(u64::MAX);
        let next = max_version.next();
        assert_eq!(next.as_u64(), u64::MAX); // Saturates, doesn't overflow
    }

    #[test]
    fn test_new_unchecked() {
        // Should create without validation (for internal use)
        let version = Version::new_unchecked(0);
        assert_eq!(version.as_u64(), 0);
    }
}
