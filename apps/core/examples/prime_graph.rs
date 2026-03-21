//! Prime Graph — build a knowledge graph and traverse it.
//!
//! Run with:
//!   cargo run --no-default-features --features prime --example prime_graph

use allsource_core::prime::{Direction, EntityId, Prime};
use serde_json::json;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Open an in-memory Prime instance (or use Prime::open("path") for persistence)
    let prime = Prime::open_in_memory().await?;

    // ─── Create nodes ───────────────────────────────────────────────────
    let alice_id = prime
        .add_node("person", json!({"name": "Alice", "role": "engineer"}))
        .await?;
    let bob_id = prime
        .add_node("person", json!({"name": "Bob", "role": "manager"}))
        .await?;
    let project_id = prime
        .add_node(
            "project",
            json!({"name": "AllSource Prime", "status": "active"}),
        )
        .await?;

    // Entity IDs include the type: "node:person:{uuid}"
    let alice = EntityId::node("person", alice_id.as_str()).to_string();
    let bob = EntityId::node("person", bob_id.as_str()).to_string();
    let project = EntityId::node("project", project_id.as_str()).to_string();

    println!("Created: Alice={alice}, Bob={bob}, Project={project}");

    // ─── Create edges ───────────────────────────────────────────────────
    prime.add_edge(&alice, &project, "works_on", None).await?;
    prime.add_edge(&bob, &project, "manages", None).await?;
    prime
        .add_edge(&bob, &alice, "mentors", Some(json!({"since": "2025-01"})))
        .await?;

    // ─── Query ──────────────────────────────────────────────────────────
    let stats = prime.stats();
    println!(
        "\nGraph: {} nodes, {} edges",
        stats.total_nodes, stats.total_edges
    );

    // Who works on the project?
    let team = prime.neighbors(&project, None, Direction::Incoming);
    println!("\nProject team:");
    for member in &team {
        println!("  - {} ({})", member.properties["name"], member.node_type);
    }

    // Who does Bob mentor?
    let mentees = prime.neighbors(&bob, Some("mentors"), Direction::Outgoing);
    println!("\nBob mentors:");
    for m in &mentees {
        println!("  - {}", m.properties["name"]);
    }

    // Shortest path from Alice to Bob
    if let Some(path) = prime.shortest_path(&alice, &bob, None) {
        println!("\nPath Alice → Bob: {} hops", path.len() - 1);
        for node in &path {
            println!("  → {}", node.properties["name"]);
        }
    }

    // ─── Update and delete ──────────────────────────────────────────────
    prime
        .update_node(
            &alice,
            json!({"level": "senior", "languages": ["Rust", "Python"]}),
        )
        .await?;

    let alice_node = prime.get_node(&alice).unwrap();
    println!(
        "\nAlice updated: level={}, languages={:?}",
        alice_node.properties["level"], alice_node.properties["languages"]
    );

    // ─── History ────────────────────────────────────────────────────────
    let history = prime.history(&alice).await?;
    println!("\nAlice's history ({} events):", history.len());
    for entry in &history {
        println!("  {} at {}", entry.event_type, entry.timestamp);
    }

    // ─── Subgraph extraction ────────────────────────────────────────────
    let sg = prime.subgraph(&alice, 2);
    println!(
        "\nAlice's 2-hop subgraph: {} nodes, {} edges",
        sg.nodes.len(),
        sg.edges.len()
    );

    prime.shutdown().await?;
    println!("\nDone.");
    Ok(())
}
