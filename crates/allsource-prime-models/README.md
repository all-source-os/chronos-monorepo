# allsource-prime-models

Vendored `all-MiniLM-L6-v2` ONNX weights for [AllSource Prime](https://github.com/all-source-os/all-source).

The five model files are fetched **once at build time** into `OUT_DIR` and
`include_bytes!`'d, so they are baked into the consuming binary. Prime's
embedder loads them via fastembed's `try_new_from_user_defined` and **never
touches the network at runtime** — making "works offline" true on a fresh
install without a manual warm or `PRIME_EMBED_MODEL_DIR`.

Enable it on `allsource-core` with the `prime-bundled-model` feature.

## Offline / air-gapped builds

By default `build.rs` downloads from HuggingFace at a pinned revision. To build
without network, vendor the five files (`model.onnx`, `tokenizer.json`,
`config.json`, `special_tokens_map.json`, `tokenizer_config.json` — e.g. from a
fastembed cache snapshot dir) and point at them:

```bash
export ALLSOURCE_PRIME_MODELS_SRC=/path/to/vendored
```

`HF_ENDPOINT=<mirror>` overrides the download host.

## Why a separate crate

`all-MiniLM-L6-v2` is ~22 MB — too large to commit to a crate within the
crates.io default size budget. Fetching at build time keeps the published
source tiny while still delivering a runtime-offline, single-artifact binary.

Model provenance: `Qdrant/all-MiniLM-L6-v2-onnx` @
`5f1b8cd78bc4fb444dd171e59b18f3a3af89a079`. 384 dims, mean-pooled, non-quantized.
