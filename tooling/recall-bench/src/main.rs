//! Benchmark harness for AllSource Recall.
//!
//! Evaluates Recall against LoCoMo and LongMemEval datasets.
//! Downloads datasets from HuggingFace on first run.
//!
//! Usage:
//!   cargo run -p recall-bench -- --dataset locomo
//!   cargo run -p recall-bench -- --dataset longmemeval
//!   cargo run -p recall-bench -- --dataset locomo --baseline vector-only

use anyhow::Result;
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub mod cross_ref;
mod datasets;
mod evaluate;

#[derive(Clone, Debug, clap::ValueEnum)]
enum Dataset {
    Locomo,
    Longmemeval,
    CrossRef,
}

#[derive(Clone, Debug, clap::ValueEnum)]
enum Baseline {
    /// Vector-only retrieval (no compressed index, no graph)
    VectorOnly,
}

#[derive(Parser)]
#[command(
    name = "recall-bench",
    about = "Benchmark AllSource Recall against standard memory evaluation datasets"
)]
struct Cli {
    /// Dataset to evaluate
    #[arg(long)]
    dataset: Dataset,

    /// Optional baseline mode for ablation study
    #[arg(long)]
    baseline: Option<Baseline>,

    /// Max conversations to evaluate (default: all)
    #[arg(long)]
    limit: Option<usize>,

    /// Data directory for downloaded datasets
    #[arg(long, default_value = ".recall-bench-data")]
    data_dir: PathBuf,

    /// Output format: table (default) or json
    #[arg(long, default_value = "table")]
    output: String,

    /// Embedding model: mini (AllMiniLML6V2, fast), base (BGEBaseENV15, better quality)
    #[arg(long, default_value = "mini")]
    model: String,
}

/// Results from a benchmark run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResults {
    pub dataset: String,
    pub mode: String,
    pub conversations: usize,
    pub queries: usize,
    pub precision: f64,
    pub recall: f64,
    pub f1: f64,
    pub cross_ref_accuracy: Option<f64>,
    pub avg_latency_ms: f64,
}

impl BenchmarkResults {
    fn print_table(&self) {
        println!("\n## {dataset} — {mode}", dataset = self.dataset, mode = self.mode);
        println!("| Metric               | Value   |");
        println!("|----------------------|---------|");
        println!("| Conversations        | {:<7} |", self.conversations);
        println!("| Queries              | {:<7} |", self.queries);
        println!("| Precision            | {:.1}%  |", self.precision * 100.0);
        println!("| Recall               | {:.1}%  |", self.recall * 100.0);
        println!("| F1                   | {:.1}%  |", self.f1 * 100.0);
        if let Some(cr) = self.cross_ref_accuracy {
            println!("| Cross-Ref Accuracy   | {:.1}%  |", cr * 100.0);
        }
        println!("| Avg Latency          | {:.1}ms |", self.avg_latency_ms);
        println!();
        println!("### zer0dex Published Baseline");
        println!("| Metric               | zer0dex |");
        println!("|----------------------|---------|");
        println!("| Cross-Ref Accuracy   | 80.0%   |");
        println!("| Recall               | 91.2%   |");
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("recall_bench=info")
        .init();

    let cli = Cli::parse();

    tracing::info!("Dataset: {:?}, Baseline: {:?}", cli.dataset, cli.baseline);

    let mode = match &cli.baseline {
        Some(Baseline::VectorOnly) => "vector-only",
        None => "full-recall",
    };

    let results = match cli.dataset {
        Dataset::Locomo => {
            evaluate::run_locomo(mode, cli.limit.unwrap_or(usize::MAX), &cli.data_dir, &cli.model).await?
        }
        Dataset::Longmemeval => {
            evaluate::run_longmemeval(mode, cli.limit.unwrap_or(usize::MAX), &cli.data_dir, &cli.model).await?
        }
        Dataset::CrossRef => {
            // Run the full cross-ref suite (3 modes: vector-only, vector+graph, full-recall)
            let all = cross_ref::run_cross_ref_suite().await?;
            // Print all modes
            for r in &all {
                if cli.output == "json" {
                    println!("{}", serde_json::to_string_pretty(&r)?);
                } else {
                    r.print_table();
                }
            }
            return Ok(());
        }
    };

    if cli.output == "json" {
        println!("{}", serde_json::to_string_pretty(&results)?);
    } else {
        results.print_table();
    }

    Ok(())
}
