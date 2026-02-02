//! # Dependency Injection Container
//!
//! Provides a centralized container for wiring up all application layers and enabling testability.
//!
//! ## Architecture
//!
//! The DI container follows the Clean Architecture principle of dependency inversion:
//! - Domain layer defines repository traits (abstractions)
//! - Infrastructure layer provides concrete implementations
//! - Application layer use cases receive dependencies via constructor injection
//! - This container wires everything together
//!
//! ## Usage
//!
//! ### Production Setup
//!
//! ```rust,ignore
//! use allsource_core::infrastructure::di::{ServiceContainer, ContainerBuilder};
//!
//! // Build container with in-memory repositories (development)
//! let container = ContainerBuilder::new()
//!     .with_in_memory_repositories()
//!     .build();
//!
//! // Get a use case with all dependencies injected
//! let register_creator = container.register_creator_use_case();
//! let response = register_creator.execute(request).await?;
//! ```
//!
//! ### Testing with Mocks
//!
//! ```rust,ignore
//! use allsource_core::infrastructure::di::{ServiceContainer, ContainerBuilder};
//!
//! // Build container with custom mock repositories
//! let container = ContainerBuilder::new()
//!     .with_creator_repository(Arc::new(MockCreatorRepository::new()))
//!     .with_article_repository(Arc::new(MockArticleRepository::new()))
//!     .build();
//!
//! // Use cases will receive mock dependencies
//! let use_case = container.register_creator_use_case();
//! ```

mod container;
mod builder;

pub use container::ServiceContainer;
pub use builder::ContainerBuilder;

#[cfg(test)]
mod tests;
