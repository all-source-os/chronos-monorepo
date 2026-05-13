//! divan wall-clock benchmarks for Prime hot paths.
//! Runs locally on any OS. Pairs with `iai.rs` for the CI regression gate.

#[cfg(not(feature = "dhat-heap"))]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use allsource_core::prime::Prime;
use serde_json::json;
use tokio::runtime::Runtime;

fn main() {
    divan::main();
}

fn rt() -> Runtime {
    Runtime::new().expect("tokio runtime")
}

async fn build_prime_with_nodes(n: usize) -> Prime {
    let prime = Prime::open_in_memory().await.expect("prime");
    for i in 0..n {
        prime
            .add_node("person", json!({ "name": format!("p{i}"), "index": i }))
            .await
            .expect("add_node");
    }
    prime
}

#[divan::bench]
fn bench_add_node(bencher: divan::Bencher) {
    let rt = rt();
    let prime = rt.block_on(Prime::open_in_memory()).expect("prime");
    let mut counter = 0usize;
    bencher.bench_local(|| {
        counter += 1;
        rt.block_on(prime.add_node(
            "person",
            json!({ "name": format!("n{counter}"), "index": counter }),
        ))
        .expect("add_node")
    });
}

#[divan::bench(args = [100usize, 1_000])]
fn bench_add_node_batch(bencher: divan::Bencher, n: usize) {
    let rt = rt();
    bencher.bench_local(|| {
        rt.block_on(async {
            let prime = Prime::open_in_memory().await.expect("prime");
            for i in 0..n {
                prime
                    .add_node("person", json!({ "name": format!("p{i}"), "index": i }))
                    .await
                    .expect("add_node");
            }
            std::hint::black_box(prime.stats());
        });
    });
}

#[divan::bench(args = [100usize, 1_000])]
fn bench_stats_over_graph(bencher: divan::Bencher, n: usize) {
    let rt = rt();
    let prime = rt.block_on(build_prime_with_nodes(n));
    bencher.bench_local(|| std::hint::black_box(prime.stats()));
}
