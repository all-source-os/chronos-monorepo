#!/bin/bash

# AllSource Demo Script
# This script demonstrates the core capabilities of AllSource

set -e

CORE_URL="http://localhost:8080"
CONTROL_URL="http://localhost:8081"

echo "╔═══════════════════════════════════════════════════════╗"
echo "║                                                       ║"
echo "║    🌟  A L L S O U R C E   D E M O                   ║"
echo "║                                                       ║"
echo "╚═══════════════════════════════════════════════════════╝"
echo ""

# Check if services are running
echo "🔍 Checking service health..."
if ! curl -s "${CORE_URL}/health" > /dev/null; then
    echo "❌ Event Store Core is not running on ${CORE_URL}"
    echo "   Start it with: cd services/core && cargo run --release"
    exit 1
fi

if ! curl -s "${CONTROL_URL}/health" > /dev/null; then
    echo "❌ Control Plane is not running on ${CONTROL_URL}"
    echo "   Start it with: cd services/control-plane && go run main.go"
    exit 1
fi

echo "✅ All services are healthy!"
echo ""

# Demo 1: Ingest events
echo "📝 Demo 1: Ingesting Events"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

USER_ID="user-$(date +%s)"
echo "Creating events for ${USER_ID}..."

# User created
echo "  → user.created"
curl -s -X POST "${CORE_URL}/api/v1/events" \
  -H "Content-Type: application/json" \
  -d "{
    \"event_type\": \"user.created\",
    \"entity_id\": \"${USER_ID}\",
    \"payload\": {
      \"name\": \"Alice Johnson\",
      \"email\": \"alice@example.com\",
      \"role\": \"engineer\"
    }
  }" | jq '.'

sleep 1

# User updated
echo "  → user.updated"
curl -s -X POST "${CORE_URL}/api/v1/events" \
  -H "Content-Type: application/json" \
  -d "{
    \"event_type\": \"user.updated\",
    \"entity_id\": \"${USER_ID}\",
    \"payload\": {
      \"role\": \"senior-engineer\",
      \"department\": \"platform\"
    }
  }" | jq '.'

sleep 1

# Order placed
ORDER_ID="order-$(date +%s)"
echo "  → order.placed for ${ORDER_ID}"
curl -s -X POST "${CORE_URL}/api/v1/events" \
  -H "Content-Type: application/json" \
  -d "{
    \"event_type\": \"order.placed\",
    \"entity_id\": \"${ORDER_ID}\",
    \"payload\": {
      \"user_id\": \"${USER_ID}\",
      \"items\": [\"laptop\", \"monitor\"],
      \"total\": 2499.99
    }
  }" | jq '.'

echo ""

# Demo 2: Query events
echo "🔍 Demo 2: Querying Events"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

echo "Querying all events for ${USER_ID}:"
curl -s "${CORE_URL}/api/v1/events/query?entity_id=${USER_ID}" | jq '.events[] | {event_type, timestamp, payload}'

echo ""

# Demo 3: State reconstruction
echo "🔄 Demo 3: State Reconstruction (Time Travel)"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

echo "Reconstructing current state for ${USER_ID}:"
curl -s "${CORE_URL}/api/v1/entities/${USER_ID}/state" | jq '.'

echo ""

# Demo 4: Statistics
echo "📊 Demo 4: Event Store Statistics"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

curl -s "${CORE_URL}/api/v1/stats" | jq '.'

echo ""

# Demo 5: Cluster status
echo "🎯 Demo 5: Cluster Status"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

curl -s "${CONTROL_URL}/api/v1/cluster/status" | jq '.'

echo ""
echo "╔═══════════════════════════════════════════════════════╗"
echo "║                                                       ║"
echo "║    ✨  Demo Complete!                                ║"
echo "║                                                       ║"
echo "║    Visit http://localhost:3000 for the Web UI        ║"
echo "║                                                       ║"
echo "╚═══════════════════════════════════════════════════════╝"
