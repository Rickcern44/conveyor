//! Persistent local decision inbox commands.

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use conveyor_core::control_plane::ControlPlaneStore;

pub(crate) fn list(directory: &Path) -> Result<String, String> {
    let decisions = ControlPlaneStore::open(directory)
        .and_then(|store| store.pending_decisions())
        .map_err(|error| error.to_string())?;
    if decisions.is_empty() {
        return Ok("decision: no pending requests".to_owned());
    }
    Ok(decisions
        .into_iter()
        .map(|decision| format!("#{} {}: {}", decision.id, decision.kind, decision.summary))
        .collect::<Vec<_>>()
        .join("\n"))
}

pub(crate) fn answer(id: i64, outcome: &str, directory: &Path) -> Result<String, String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_secs() as i64;
    let updated = ControlPlaneStore::open(directory)
        .and_then(|store| store.resolve_decision(id, outcome, now))
        .map_err(|error| error.to_string())?;
    if updated {
        Ok(format!("decision #{id} recorded as {outcome}"))
    } else {
        Err(format!("decision #{id} is not pending"))
    }
}
