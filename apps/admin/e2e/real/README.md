# Admin E2E Tests — Real Stack

These tests run against a real Docker stack (Core + Control Plane + Admin).
No API mocking — every request hits real services.

## Prerequisites

1. Docker running
2. The test stack started:
   ```bash
   docker compose -f e2e/real/docker-compose.e2e.yml up -d --wait
   ```
3. Playwright browsers installed:
   ```bash
   bunx playwright install chromium
   ```

## Running

```bash
# Start the stack, run tests, tear down
bun run test:e2e:real

# Or manually
docker compose -f e2e/real/docker-compose.e2e.yml up -d --wait
ADMIN_BASE_URL=http://localhost:3011 bunx playwright test --config=e2e/real/playwright.config.ts
docker compose -f e2e/real/docker-compose.e2e.yml down -v
```

## Architecture

```
Browser → Admin App (port 3011)
            ├─ Middleware: checks admin_token cookie (JWT decode, role=admin)
            ├─ /api/auth/session → validates token with Control Plane
            └─ Frontend fetches → Control Plane (port 3091, /api/v1/admin/*)
                                     └─ Core (port 3090, /api/v1/*)
```

## Auth strategy

The admin middleware decodes the JWT from the `admin_token` cookie without
signature verification (it only checks expiry + `role: "admin"`). The tests
generate a valid HS256 JWT signed with the same secret as the Control Plane
and inject it as a cookie via Playwright's `storageState`.

## Seeding

The `global-setup.ts` script:
1. Waits for Core and Control Plane health checks
2. Generates an admin JWT (HS256, same secret as Control Plane)
3. Seeds a test tenant + events via Core API
4. Writes Playwright storage state with the admin cookie
