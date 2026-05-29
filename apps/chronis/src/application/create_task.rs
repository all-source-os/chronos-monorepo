use crate::{
    domain::{error::ChronError, repository::TaskRepository, task::TaskType},
    infrastructure::id::generate_task_id,
};

pub struct CreateTaskInput<'a> {
    pub title: &'a str,
    pub priority: &'a str,
    pub blocked_by: &'a [String],
    pub task_type: TaskType,
    pub parent: Option<&'a str>,
    pub description: Option<&'a str>,
}

#[derive(Debug)]
pub struct CreateTaskOutput {
    /// The ID the task was *actually* persisted under. May differ from the
    /// first candidate ID if a collision forced a retry — callers MUST report
    /// this value, never a locally-generated guess (issue #194).
    pub id: String,
}

/// How many fresh IDs we try before giving up. Collisions should be vanishingly
/// rare with 24 bits of entropy, so any exhaustion here signals something is
/// badly wrong (e.g. a stuck/degenerate RNG) and we fail loudly rather than
/// risk mutating an existing record.
const MAX_ID_ATTEMPTS: usize = 8;

#[cfg_attr(feature = "hotpath", hotpath::measure)]
pub async fn create_task(
    repo: &impl TaskRepository,
    input: CreateTaskInput<'_>,
) -> Result<CreateTaskOutput, ChronError> {
    create_task_with_id_gen(repo, input, generate_task_id).await
}

/// Collision-safe create with an injectable ID generator (for tests).
///
/// Generates a candidate ID and asks the repository to create the task. The
/// repository rejects any ID that already maps to a projected task
/// (`ChronError::IdAlreadyTaken`) *before* emitting a single event, so a
/// collision never mutates or leaks fields onto the existing record. On
/// collision we retry with a fresh ID up to `MAX_ID_ATTEMPTS`; if every
/// candidate is taken we fail with `ChronError::IdCollisionExhausted` and a
/// non-zero exit — we never fall back to mutating an existing task.
///
/// The returned `id` is the one the task was genuinely persisted under.
pub async fn create_task_with_id_gen(
    repo: &impl TaskRepository,
    input: CreateTaskInput<'_>,
    mut gen_id: impl FnMut() -> String,
) -> Result<CreateTaskOutput, ChronError> {
    for _ in 0..MAX_ID_ATTEMPTS {
        let id = gen_id();
        match repo
            .create_task(
                &id,
                input.title,
                input.priority,
                input.blocked_by,
                input.task_type,
                input.parent,
                input.description,
            )
            .await
        {
            Ok(()) => return Ok(CreateTaskOutput { id }),
            // Collision: the candidate ID is taken. The repository guaranteed
            // no events were emitted against it — try a fresh ID.
            Err(ChronError::IdAlreadyTaken(_)) => continue,
            Err(e) => return Err(e),
        }
    }
    Err(ChronError::IdCollisionExhausted(MAX_ID_ATTEMPTS))
}
