#!/usr/bin/env bash
# Seed the Prime demo instance with the revenue/engineering/product dataset.
# Run against a running Prime HTTP server.
#
# Usage: ./scripts/seed-demo.sh [base_url]
# Default: http://localhost:3905

set -euo pipefail

BASE="${1:-http://localhost:3905}"
API="$BASE/api/v1/prime"

echo "Seeding Prime demo at $BASE"

# Check health
if ! curl -sf "$BASE/health" > /dev/null 2>&1; then
  echo "ERROR: Prime not reachable at $BASE"
  exit 1
fi

# Helper: create node, capture entity_id
create_node() {
  local type="$1" name="$2" domain="$3"
  local extra="${4:-}"
  local props="{\"name\": \"$name\", \"domain\": \"$domain\"$extra}"
  curl -sf -X POST "$API/nodes" \
    -H 'Content-Type: application/json' \
    -d "{\"type\": \"$type\", \"properties\": $props}" \
    | grep -o '"entity_id":"[^"]*"' | cut -d'"' -f4
}

# Helper: create edge
create_edge() {
  local source="$1" target="$2" relation="$3"
  curl -sf -X POST "$API/edges" \
    -H 'Content-Type: application/json' \
    -d "{\"source\": \"$source\", \"target\": \"$target\", \"relation\": \"$relation\"}" \
    > /dev/null
}

echo ""
echo "=== Revenue domain ==="
REV1=$(create_node "metric" "Q3 Revenue" "revenue" ', "value": "$4.2M", "trend": "up 15%"')
echo "  $REV1"
REV2=$(create_node "metric" "Churn Rate" "revenue" ', "value": "8.5%", "segment": "SMB"')
echo "  $REV2"
REV3=$(create_node "metric" "ARR" "revenue" ', "value": "$16M"')
echo "  $REV3"
REV4=$(create_node "decision" "June Pricing Change" "revenue" ', "impact": "enterprise growth"')
echo "  $REV4"
REV5=$(create_node "insight" "SMB Churn Pattern" "revenue" ', "finding": "churn concentrated under $10K ACV"')
echo "  $REV5"

echo ""
echo "=== Engineering domain ==="
ENG1=$(create_node "service" "Core API" "engineering" ', "throughput": "469K events/sec"')
echo "  $ENG1"
ENG2=$(create_node "feature" "Replication" "engineering" ', "type": "leader-follower"')
echo "  $ENG2"
ENG3=$(create_node "metric" "Build Time" "engineering" ', "value": "2m05s", "trend": "improved"')
echo "  $ENG3"
ENG4=$(create_node "feature" "Prime Engine" "engineering" ', "status": "shipped"')
echo "  $ENG4"
ENG5=$(create_node "metric" "Test Coverage" "engineering" ', "value": "87%"')
echo "  $ENG5"

echo ""
echo "=== Product domain ==="
PROD1=$(create_node "feature" "Dark Mode" "product" ', "version": "v3.2"')
echo "  $PROD1"
PROD2=$(create_node "metric" "NPS Score" "product" ', "value": "58", "trend": "up from 42"')
echo "  $PROD2"
PROD3=$(create_node "insight" "User Research" "product" ', "finding": "70% want better search"')
echo "  $PROD3"
PROD4=$(create_node "feature" "Schema Registry" "product" ', "status": "shipped"')
echo "  $PROD4"

echo ""
echo "=== Security domain ==="
SEC1=$(create_node "feature" "API Key Auth" "security" ', "hash": "SHA-256"')
echo "  $SEC1"
SEC2=$(create_node "policy" "TLS 1.3" "security" ', "scope": "all external connections"')
echo "  $SEC2"
SEC3=$(create_node "event" "SOC 2 Audit" "security" ', "result": "passed", "date": "2026-02"')
echo "  $SEC3"

echo ""
echo "=== Cross-domain relationships ==="
create_edge "$REV4" "$ENG1" "requires"
echo "  pricing → core API (requires)"
create_edge "$ENG3" "$REV1" "improves"
echo "  build time → revenue (improves)"
create_edge "$PROD2" "$REV1" "drives"
echo "  NPS → revenue (drives)"
create_edge "$PROD3" "$ENG4" "depends_on"
echo "  user research → prime engine (depends_on)"
create_edge "$ENG5" "$SEC3" "validates"
echo "  test coverage → SOC 2 (validates)"
create_edge "$SEC1" "$ENG1" "protects"
echo "  API key auth → core API (protects)"
create_edge "$REV5" "$PROD1" "leads_to"
echo "  SMB churn → dark mode (leads_to)"
create_edge "$PROD4" "$SEC2" "requires"
echo "  schema registry → TLS (requires)"

echo ""
echo "=== Stats ==="
curl -sf "$API/stats" | python3 -m json.tool 2>/dev/null || curl -sf "$API/stats"

echo ""
echo "Done. $(date)"
