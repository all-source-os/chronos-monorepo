# SDK Distribution Setup Checklist

Manual steps to set up the static R2 SDK registry (`registry.all-source.xyz`).

See `docs/proposals/SDK_REGISTRY_DESIGN.md` for full design.

## 1. Cloudflare R2 Setup

- [ ] Create R2 bucket `allsource-registry` in Cloudflare dashboard
- [ ] Enable bucket versioning (Settings → Object versioning)
- [ ] Attach custom domain `registry.all-source.xyz` (Settings → Custom domains)
- [ ] Create scoped API token: R2 → Object Read & Write → bucket `allsource-registry` only

## 2. GitHub Secrets

Add to `all-source-os/allsource-monorepo` → Settings → Secrets → Actions:

- [ ] `R2_ACCESS_KEY_ID` — from the scoped API token above
- [ ] `R2_SECRET_ACCESS_KEY` — from the scoped API token above
- [ ] `R2_ENDPOINT` — `https://<account-id>.r2.cloudflarestorage.com`

## 3. Build the Metadata Script

- [ ] Create `scripts/generate-registry-metadata.sh`
- [ ] Test locally: `VERSION=0.10.5 ./scripts/generate-registry-metadata.sh`
- [ ] Verify `dist/registry/` layout matches the R2 bucket structure in the proposal

## 4. Update CI

- [ ] Replace `publish-sdks` job in `.github/workflows/release.yml` with `upload-registry` job
- [ ] Install rclone in CI, configure R2 as remote, `rclone sync dist/registry/ r2:allsource-registry/`

## 5. Seed Initial Data

- [ ] Run metadata script for v0.10.5
- [ ] Upload to R2: `rclone sync dist/registry/ r2:allsource-registry/`
- [ ] Verify each protocol works:
  ```bash
  # Cargo
  curl https://registry.all-source.xyz/cargo/config.json
  curl https://registry.all-source.xyz/cargo/al/ls/allsource

  # Go
  curl https://registry.all-source.xyz/go/github.com/all-source-os/allsource-go/@v/list

  # npm
  curl https://registry.all-source.xyz/npm/@allsource/client

  # PyPI
  curl https://registry.all-source.xyz/pypi/simple/allsource-client/
  ```

## 6. Test Consumer Install

- [ ] Rust:
  ```bash
  # Add to .cargo/config.toml
  [registries.allsource]
  index = "sparse+https://registry.all-source.xyz/cargo/"

  cargo add allsource --registry allsource
  ```
- [ ] Go:
  ```bash
  GOPROXY=https://registry.all-source.xyz/go,https://proxy.golang.org,direct \
    go get github.com/all-source-os/allsource-go@v0.10.5
  ```
- [ ] TypeScript:
  ```bash
  echo '@allsource:registry=https://registry.all-source.xyz/npm/' >> .npmrc
  npm install @allsource/client
  ```
- [ ] Python:
  ```bash
  pip install allsource-client --index-url https://registry.all-source.xyz/pypi/simple/
  ```

## 7. Documentation

- [ ] Add registry setup instructions to each SDK README
- [ ] Add installation section to main README
