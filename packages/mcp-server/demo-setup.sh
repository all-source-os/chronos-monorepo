#!/bin/bash

# AllSource MCP Demo Setup Script
# This script populates the event store with realistic demo data

set -e

CORE_URL="http://localhost:8080"

echo "╔═══════════════════════════════════════════════════════╗"
echo "║                                                       ║"
echo "║    🎪  AllSource MCP Demo Data Setup                ║"
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

echo "✅ AllSource Core is healthy!"
echo ""

# Create demo users with rich event history
echo "👤 Creating demo users..."

USERS=("alice" "bob" "charlie" "diana" "evan")
USER_IDS=()

for user in "${USERS[@]}"; do
    USER_ID="user-$(date +%s)-${user}"
    USER_IDS+=("${USER_ID}")

    echo "  → Creating ${user} (${USER_ID})"

    # User created
    curl -s -X POST "${CORE_URL}/api/v1/events" \
      -H "Content-Type: application/json" \
      -d "{
        \"event_type\": \"user.created\",
        \"entity_id\": \"${USER_ID}\",
        \"payload\": {
          \"name\": \"${user^}\",
          \"email\": \"${user}@example.com\",
          \"role\": \"engineer\",
          \"department\": \"engineering\",
          \"salary\": 100000,
          \"hire_date\": \"2024-01-01\"
        }
      }" > /dev/null

    sleep 0.1

    # User profile updated
    curl -s -X POST "${CORE_URL}/api/v1/events" \
      -H "Content-Type: application/json" \
      -d "{
        \"event_type\": \"user.profile_updated\",
        \"entity_id\": \"${USER_ID}\",
        \"payload\": {
          \"phone\": \"+1-555-0$((RANDOM % 10))00-$((RANDOM % 10))000\",
          \"timezone\": \"America/New_York\"
        }
      }" > /dev/null

    sleep 0.1

    # Some users got promoted
    if [ "${user}" == "alice" ] || [ "${user}" == "bob" ]; then
        curl -s -X POST "${CORE_URL}/api/v1/events" \
          -H "Content-Type: application/json" \
          -d "{
            \"event_type\": \"user.promoted\",
            \"entity_id\": \"${USER_ID}\",
            \"payload\": {
              \"old_role\": \"engineer\",
              \"new_role\": \"senior-engineer\",
              \"salary\": 120000,
              \"promotion_date\": \"2024-06-01\"
            }
          }" > /dev/null

        sleep 0.1
    fi

    # Login events
    for i in {1..3}; do
        curl -s -X POST "${CORE_URL}/api/v1/events" \
          -H "Content-Type: application/json" \
          -d "{
            \"event_type\": \"user.login\",
            \"entity_id\": \"${USER_ID}\",
            \"payload\": {
              \"ip_address\": \"192.168.1.$((RANDOM % 255))\",
              \"user_agent\": \"Mozilla/5.0\",
              \"timestamp\": \"$(date -u +%Y-%m-%dT%H:%M:%SZ)\"
            }
          }" > /dev/null

        sleep 0.05
    done
done

echo ""
echo "📦 Creating demo orders..."

# Create some orders
for i in {1..5}; do
    ORDER_ID="order-$(date +%s)-${i}"
    USER_ID="${USER_IDS[$((RANDOM % ${#USER_IDS[@]}))]}"

    echo "  → Creating ${ORDER_ID}"

    # Order created
    curl -s -X POST "${CORE_URL}/api/v1/events" \
      -H "Content-Type: application/json" \
      -d "{
        \"event_type\": \"order.created\",
        \"entity_id\": \"${ORDER_ID}\",
        \"payload\": {
          \"user_id\": \"${USER_ID}\",
          \"items\": [
            {\"product\": \"Laptop\", \"price\": 1200, \"quantity\": 1},
            {\"product\": \"Mouse\", \"price\": 50, \"quantity\": 2}
          ],
          \"total\": 1300,
          \"status\": \"pending\"
        }
      }" > /dev/null

    sleep 0.1

    # Order paid
    curl -s -X POST "${CORE_URL}/api/v1/events" \
      -H "Content-Type: application/json" \
      -d "{
        \"event_type\": \"order.payment_received\",
        \"entity_id\": \"${ORDER_ID}\",
        \"payload\": {
          \"payment_method\": \"credit_card\",
          \"amount\": 1300,
          \"status\": \"paid\"
        }
      }" > /dev/null

    sleep 0.1

    # Some orders shipped
    if [ $((RANDOM % 2)) -eq 0 ]; then
        curl -s -X POST "${CORE_URL}/api/v1/events" \
          -H "Content-Type: application/json" \
          -d "{
            \"event_type\": \"order.shipped\",
            \"entity_id\": \"${ORDER_ID}\",
            \"payload\": {
              \"tracking_number\": \"TRACK-$((RANDOM % 10000))\",
              \"carrier\": \"FedEx\",
              \"status\": \"shipped\"
            }
          }" > /dev/null

        sleep 0.1
    fi
done

echo ""
echo "📊 Demo data summary..."

curl -s "${CORE_URL}/api/v1/stats" | jq '.'

echo ""
echo "╔═══════════════════════════════════════════════════════╗"
echo "║                                                       ║"
echo "║    ✨  Demo Data Ready!                              ║"
echo "║                                                       ║"
echo "║    Try these queries in Claude Desktop:              ║"
echo "║                                                       ║"
echo "║    • \"Show me all users\"                            ║"
echo "║    • \"What did alice look like when first created?\" ║"
echo "║    • \"Compare alice and bob\"                        ║"
echo "║    • \"Find patterns in login events\"                ║"
echo "║    • \"Explain everything about an order\"            ║"
echo "║                                                       ║"
echo "╚═══════════════════════════════════════════════════════╝"
echo ""
echo "💡 Tip: Save the entity IDs above to use in your queries!"
echo ""
