package main

import (
	"go/ast"
	"go/parser"
	"go/token"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// statusProbeFuncs are the Control Plane's status / liveness / readiness / health
// probe handlers. NONE of them may reference /api/v1/demo/* — minting a tenant
// from a health check is the defect that littered production /tenants with "Demo
// User" tenants on nearly every status poll (gap #1, ADMIN_TENANT_POWER_TOOL.md
// §3). The DemoStartHandler is now gated behind DEMO_ENABLED, but this test makes
// the class of bug structurally impossible to reintroduce: if anyone wires a
// probe to /demo/start again (directly or via a helper string), CI fails here.
//
// This is the in-app half of the rule. The web status-probe fix
// (apps/web/.../status/services/route.ts) is a separate prompt; this guard owns
// the Control Plane side.
var statusProbeFuncs = map[string]bool{
	"livezHandler":          true, // GET /livez — liveness (Fly machine check)
	"healthHandler":         true, // GET /health — readiness
	"coreHealthHandler":     true, // GET /api/v1/health/core
	"statusServicesHandler": true, // GET /api/v1/status/services — public status feed
	"currentServiceStatus":  true, // the projection the status feed renders
}

// demoNeedle is the path fragment a status path must never reference.
const demoNeedle = "/demo"

// TestNoStatusPathReferencesDemo parses every Control Plane .go source file and
// asserts that none of the status/liveness/health probe functions contains a
// string literal referencing /demo. It scans function BODIES (not whole files),
// so the legitimate /api/v1/demo route registration in setupRoutes is allowed
// while a probe that calls it is not.
func TestNoStatusPathReferencesDemo(t *testing.T) {
	fset := token.NewFileSet()

	entries, err := os.ReadDir(".")
	if err != nil {
		t.Fatalf("read control-plane dir: %v", err)
	}

	foundFuncs := map[string]bool{}

	for _, e := range entries {
		name := e.Name()
		if e.IsDir() || !strings.HasSuffix(name, ".go") || strings.HasSuffix(name, "_test.go") {
			continue
		}
		path := filepath.Clean(name)
		file, perr := parser.ParseFile(fset, path, nil, 0)
		if perr != nil {
			t.Fatalf("parse %s: %v", path, perr)
		}

		for _, decl := range file.Decls {
			fn, ok := decl.(*ast.FuncDecl)
			if !ok || fn.Body == nil {
				continue
			}
			if !statusProbeFuncs[fn.Name.Name] {
				continue
			}
			foundFuncs[fn.Name.Name] = true

			// Walk the function body for any string literal mentioning /demo.
			ast.Inspect(fn.Body, func(n ast.Node) bool {
				lit, ok := n.(*ast.BasicLit)
				if !ok || lit.Kind != token.STRING {
					return true
				}
				if strings.Contains(lit.Value, demoNeedle) {
					pos := fset.Position(lit.Pos())
					t.Errorf("status/liveness probe %s references %q at %s:%d — "+
						"a health check must NEVER call /api/v1/demo/* (it mints a tenant). "+
						"Use a non-mutating probe.", fn.Name.Name, lit.Value, path, pos.Line)
				}
				return true
			})
		}
	}

	// Guard against silent drift: if a probe function is renamed/removed, this
	// test would otherwise pass vacuously. Require that the load-bearing probes
	// are still present and were actually scanned.
	for _, must := range []string{"livezHandler", "healthHandler", "statusServicesHandler"} {
		if !foundFuncs[must] {
			t.Errorf("expected to scan status probe %q but it was not found — "+
				"if it was renamed, update statusProbeFuncs so the no-demo guard keeps covering it", must)
		}
	}
}
