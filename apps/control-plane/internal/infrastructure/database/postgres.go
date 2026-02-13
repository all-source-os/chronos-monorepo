// Package database provides PostgreSQL connection pool, migration runner, and health checks.
package database

import (
	"context"
	"fmt"
	"log"
	"os"
	"path/filepath"
	"sort"
	"strings"

	"github.com/jackc/pgx/v5/pgxpool"
)

// PoolStats contains connection pool statistics for health reporting.
type PoolStats struct {
	TotalConns        int32 `json:"total_conns"`
	IdleConns         int32 `json:"idle_conns"`
	AcquiredConns     int32 `json:"acquired_conns"`
	MaxConns          int32 `json:"max_conns"`
	AcquireCount      int64 `json:"acquire_count"`
	EmptyAcquireCount int64 `json:"empty_acquire_count"`
}

// Postgres wraps a pgxpool.Pool and provides migration and health check functionality.
type Postgres struct {
	Pool *pgxpool.Pool
}

// New creates a new Postgres instance connected via DATABASE_URL.
func New(ctx context.Context) (*Postgres, error) {
	databaseURL := os.Getenv("DATABASE_URL")
	if databaseURL == "" {
		return nil, fmt.Errorf("DATABASE_URL environment variable is required")
	}

	pool, err := pgxpool.New(ctx, databaseURL)
	if err != nil {
		return nil, fmt.Errorf("failed to create connection pool: %w", err)
	}

	if err := pool.Ping(ctx); err != nil {
		pool.Close()
		return nil, fmt.Errorf("failed to ping database: %w", err)
	}

	return &Postgres{Pool: pool}, nil
}

// Close closes the connection pool.
func (pg *Postgres) Close() {
	pg.Pool.Close()
}

// HealthCheck returns pool statistics for monitoring.
func (pg *Postgres) HealthCheck() PoolStats {
	stat := pg.Pool.Stat()
	return PoolStats{
		TotalConns:        stat.TotalConns(),
		IdleConns:         stat.IdleConns(),
		AcquiredConns:     stat.AcquiredConns(),
		MaxConns:          stat.MaxConns(),
		AcquireCount:      stat.AcquireCount(),
		EmptyAcquireCount: stat.EmptyAcquireCount(),
	}
}

// RunMigrations reads .sql files from the given directory and applies them in sorted order.
// It creates a schema_migrations tracking table to avoid re-applying migrations.
func (pg *Postgres) RunMigrations(ctx context.Context, migrationsDir string) error {
	// Create the tracking table if it doesn't exist
	_, err := pg.Pool.Exec(ctx, `
		CREATE TABLE IF NOT EXISTS schema_migrations (
			version TEXT PRIMARY KEY,
			applied_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
		)
	`)
	if err != nil {
		return fmt.Errorf("failed to create schema_migrations table: %w", err)
	}

	// List migration files
	entries, err := os.ReadDir(migrationsDir)
	if err != nil {
		return fmt.Errorf("failed to read migrations directory %s: %w", migrationsDir, err)
	}

	var files []string
	for _, entry := range entries {
		if !entry.IsDir() && strings.HasSuffix(entry.Name(), ".sql") {
			files = append(files, entry.Name())
		}
	}
	sort.Strings(files)

	for _, file := range files {
		version := strings.TrimSuffix(file, ".sql")

		// Check if already applied
		var exists bool
		err := pg.Pool.QueryRow(ctx,
			"SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version = $1)", version,
		).Scan(&exists)
		if err != nil {
			return fmt.Errorf("failed to check migration %s: %w", version, err)
		}
		if exists {
			continue
		}

		// Read and execute migration
		content, err := os.ReadFile(filepath.Join(migrationsDir, file))
		if err != nil {
			return fmt.Errorf("failed to read migration %s: %w", file, err)
		}

		log.Printf("Applying migration: %s", file)

		tx, err := pg.Pool.Begin(ctx)
		if err != nil {
			return fmt.Errorf("failed to begin transaction for %s: %w", file, err)
		}

		if _, err := tx.Exec(ctx, string(content)); err != nil {
			_ = tx.Rollback(ctx)
			return fmt.Errorf("failed to apply migration %s: %w", file, err)
		}

		if _, err := tx.Exec(ctx, "INSERT INTO schema_migrations (version) VALUES ($1)", version); err != nil {
			_ = tx.Rollback(ctx)
			return fmt.Errorf("failed to record migration %s: %w", file, err)
		}

		if err := tx.Commit(ctx); err != nil {
			return fmt.Errorf("failed to commit migration %s: %w", file, err)
		}

		log.Printf("Applied migration: %s", file)
	}

	return nil
}
