use rand::RngExt;

/// Generate a short, human-friendly task ID like `t-a3f1`.
pub fn generate_task_id() -> String {
    let n: u16 = rand::rng().random();
    format!("t-{n:04x}")
}
