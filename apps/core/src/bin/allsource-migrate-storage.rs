//! One-shot CLI for the flat → tenant-partitioned storage migration.
//!
//! Run with Core stopped. See docs/migrations/STORAGE_LAYOUT_FLAT_TO_TENANT.md
//! for the operational playbook.
//!
//! Usage:
//!   allsource-migrate-storage --storage-dir /app/data/storage [--dry-run]

use allsource_core::infrastructure::persistence::ParquetStorage;
use std::process::ExitCode;

fn print_usage() {
    eprintln!(
        "Usage: allsource-migrate-storage --storage-dir <PATH> [--dry-run]\n\n\
         Migrates legacy flat-layout Parquet files\n\
           <storage-dir>/events-*.parquet\n\
         into the tenant-partitioned tree\n\
           <storage-dir>/<tenant>/<yyyy-mm>/events-*.parquet\n\n\
         The migration runs entirely on the local filesystem. Stop Core\n\
         before running so no concurrent writes race the move.\n\n\
         Run with --dry-run first to preview what would happen.\n"
    );
}

fn main() -> ExitCode {
    let mut storage_dir: Option<String> = None;
    let mut dry_run = false;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--storage-dir" => {
                storage_dir = args.next();
            }
            "--dry-run" => {
                dry_run = true;
            }
            "-h" | "--help" => {
                print_usage();
                return ExitCode::SUCCESS;
            }
            other => {
                eprintln!("error: unrecognized argument {other:?}\n");
                print_usage();
                return ExitCode::from(2);
            }
        }
    }

    let Some(storage_dir) = storage_dir else {
        eprintln!("error: --storage-dir is required\n");
        print_usage();
        return ExitCode::from(2);
    };

    let storage = match ParquetStorage::new(&storage_dir) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: failed to open storage at {storage_dir}: {e}");
            return ExitCode::from(1);
        }
    };

    if dry_run {
        eprintln!("[dry-run] scanning {storage_dir} ...");
    } else {
        eprintln!("migrating {storage_dir} (Core MUST be stopped before running)");
    }

    let report = match storage.migrate_flat_layout(dry_run) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: migration failed: {e}");
            return ExitCode::from(1);
        }
    };

    println!(
        "dry_run            : {}\n\
         flat_files_seen    : {}\n\
         flat_files_removed : {}\n\
         partitions_written : {}\n\
         events_migrated    : {}",
        report.dry_run,
        report.flat_files_seen,
        report.flat_files_removed,
        report.partitions_written,
        report.events_migrated,
    );

    if dry_run && report.flat_files_seen > 0 {
        eprintln!(
            "\n[dry-run] {} legacy file(s) would be migrated. Re-run without --dry-run to apply.",
            report.flat_files_seen
        );
    }

    ExitCode::SUCCESS
}
