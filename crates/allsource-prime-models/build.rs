//! Build-time fetch of the all-MiniLM-L6-v2 model files into `OUT_DIR`.
//!
//! `src/lib.rs` then `include_bytes!`s them, so the weights are baked into the
//! consuming binary and Prime's embedder never touches the network at runtime.
//!
//! Two sources, in order:
//!   1. `ALLSOURCE_PRIME_MODELS_SRC=<dir>` — copy the five files from a local
//!      directory. Lets you build fully offline (CI, air-gapped) by vendoring
//!      the files once (e.g. from a fastembed cache snapshot dir).
//!   2. Otherwise download from HuggingFace at a pinned revision. `HF_ENDPOINT`
//!      overrides the host for mirrors.
//!
//! Downloads are cached in `OUT_DIR`, so a clean rebuild re-fetches but an
//! incremental one does not.

use std::{
    env, fs,
    io::Read,
    path::{Path, PathBuf},
};

/// HuggingFace repo holding the ONNX export.
const REPO: &str = "Qdrant/all-MiniLM-L6-v2-onnx";
/// Pinned revision (commit sha) for reproducible, supply-chain-stable builds.
/// Matches the snapshot fastembed resolves for `AllMiniLML6V2`.
const REVISION: &str = "5f1b8cd78bc4fb444dd171e59b18f3a3af89a079";

/// The five files fastembed needs to build the embedder from bytes.
const FILES: &[&str] = &[
    "model.onnx",
    "tokenizer.json",
    "config.json",
    "special_tokens_map.json",
    "tokenizer_config.json",
];

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR set by cargo"));

    println!("cargo:rerun-if-env-changed=ALLSOURCE_PRIME_MODELS_SRC");
    println!("cargo:rerun-if-env-changed=HF_ENDPOINT");
    println!("cargo:rerun-if-changed=build.rs");

    let local_src = env::var("ALLSOURCE_PRIME_MODELS_SRC")
        .ok()
        .filter(|s| !s.trim().is_empty());
    let endpoint = env::var("HF_ENDPOINT").unwrap_or_else(|_| "https://huggingface.co".to_string());

    for file in FILES {
        let dst = out_dir.join(file);
        if dst.exists() && fs::metadata(&dst).map(|m| m.len() > 0).unwrap_or(false) {
            continue; // already materialized in this OUT_DIR
        }

        let bytes = match &local_src {
            Some(dir) => {
                let path = Path::new(dir).join(file);
                fs::read(&path).unwrap_or_else(|e| {
                    panic!(
                        "ALLSOURCE_PRIME_MODELS_SRC={dir} is set but {} could not be read: {e}. \
                         Provide all five files: {}.",
                        path.display(),
                        FILES.join(", ")
                    )
                })
            }
            None => {
                let url = format!("{endpoint}/{REPO}/resolve/{REVISION}/{file}");
                download(&url).unwrap_or_else(|e| {
                    panic!(
                        "failed to fetch {url}: {e}\n\
                         To build offline, vendor the five model files and set \
                         ALLSOURCE_PRIME_MODELS_SRC=<dir> (files: {}). \
                         HF_ENDPOINT=<mirror> overrides the host.",
                        FILES.join(", ")
                    )
                })
            }
        };

        fs::write(&dst, &bytes)
            .unwrap_or_else(|e| panic!("could not write {}: {e}", dst.display()));
    }
}

/// Download a URL into a byte vector, following redirects (HF serves from a CDN).
fn download(url: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let resp = ureq::get(url)
        .timeout(std::time::Duration::from_secs(300))
        .call()?;
    let mut buf = Vec::new();
    resp.into_reader().read_to_end(&mut buf)?;
    if buf.is_empty() {
        return Err("empty response body".into());
    }
    Ok(buf)
}
