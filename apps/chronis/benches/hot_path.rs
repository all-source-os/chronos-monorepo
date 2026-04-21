//! divan wall-clock benchmarks for chronis hot paths.
//!
//! Mirrors the representative CLI workload in PERF_NOTES.md:
//!   init → create N tasks → list → done
//! but drives the library API directly to avoid argument-parsing variance.

#[cfg(not(feature = "dhat-heap"))]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use std::sync::Arc;

use allsource_core::embedded::{Config, EmbeddedCore};
use chronis::{
    domain::{repository::TaskRepository, task::TaskType},
    infrastructure::{
        backend::CoreBackend, core_task_repo::CoreTaskRepository, projection::TaskProjection,
    },
};
use tokio::runtime::Runtime;

fn main() {
    divan::main();
}

fn rt() -> Runtime {
    Runtime::new().expect("tokio runtime")
}

async fn setup() -> CoreTaskRepository {
    let config = Config::builder()
        .single_tenant(true)
        .build()
        .expect("config");
    let core = Arc::new(EmbeddedCore::open(config).await.expect("core"));
    core.inner()
        .register_projection_with_backfill(
            &(Arc::new(TaskProjection::new()) as Arc<dyn allsource_core::application::Projection>),
        )
        .expect("projection");
    let backend = Arc::new(CoreBackend::new_embedded(core));
    CoreTaskRepository::new(backend)
}

#[divan::bench]
fn bench_workspace_init(bencher: divan::Bencher) {
    let rt = rt();
    bencher.bench_local(|| {
        std::hint::black_box(rt.block_on(setup()));
    });
}

#[divan::bench]
fn bench_create_single_task(bencher: divan::Bencher) {
    let rt = rt();
    let repo = rt.block_on(setup());
    let mut counter = 0usize;
    bencher.bench_local(|| {
        counter += 1;
        rt.block_on(repo.create_task(
            &format!("t-{counter:08x}"),
            &format!("task {counter}"),
            "p2",
            &[],
            TaskType::Task,
            None,
            None,
        ))
        .expect("create_task");
    });
}

#[divan::bench(args = [10usize, 100])]
fn bench_create_then_list(bencher: divan::Bencher, n: usize) {
    let rt = rt();
    bencher.bench_local(|| {
        rt.block_on(async {
            let repo = setup().await;
            for i in 0..n {
                repo.create_task(
                    &format!("t-{i:08x}"),
                    &format!("task {i}"),
                    "p2",
                    &[],
                    TaskType::Task,
                    None,
                    None,
                )
                .await
                .expect("create");
            }
            std::hint::black_box(repo.list_tasks(None).expect("list"));
        });
    });
}
