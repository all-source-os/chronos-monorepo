use rand::RngExt;

/// Generate a short, human-friendly task ID like `t-a3f1c2`.
///
/// 24 bits of entropy = 6 lowercase-hex chars (`t-xxxxxx`), for ~16.7M
/// distinct IDs. The previous allocator used a `u16` (4 hex chars / 65,536
/// IDs), which started colliding at a few hundred tasks and — combined with
/// the merge-on-collision projection — caused the silent data loss /
/// dependency-leak in issue #194.
///
/// Back-compat: the shape is unchanged — `t-` prefix, lowercase hex. Existing
/// 4-char IDs (`t-a3f1`) remain valid and parseable; grep patterns should be
/// widened from `t-[a-z0-9]{4}` to `t-[a-z0-9]{4,6}` (or `t-[a-z0-9]+`).
///
/// Collision-handling remains the caller's responsibility (see
/// `CoreTaskRepository::create_task` and `application::create_task`): the
/// wider space here only lowers the collision *probability*, it does not
/// guarantee uniqueness on its own.
pub fn generate_task_id() -> String {
    // Mask a u32 down to 24 bits, then render as fixed-width 6-char hex.
    let n: u32 = rand::rng().random::<u32>() & 0x00ff_ffff;
    format!("t-{n:06x}")
}
