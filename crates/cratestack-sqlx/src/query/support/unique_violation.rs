//! Per-item write-error classifier shared by `batch_create`,
//! `batch_update`, and `batch_upsert`.
//!
//! Each of those runs its terminal INSERT/UPDATE/UPSERT inside a
//! per-item SAVEPOINT and needs a unique-constraint violation
//! (SQLSTATE 23505) to surface as `CoolError::Conflict` so the batch
//! envelope's per-item error code matches the single-row create/update
//! paths' semantics. Anything else preserves sqlstate/constraint via
//! [`cool_error_from_sqlx`].

use cratestack_core::CoolError;

use crate::{cool_error_from_sqlx, sqlx};

pub(crate) fn classify_unique_violation(error: sqlx::Error) -> CoolError {
    if let sqlx::Error::Database(db_err) = &error
        && let Some(code) = db_err.code()
        && code == "23505"
    {
        return CoolError::Conflict(db_err.message().to_owned());
    }
    cool_error_from_sqlx(error)
}
