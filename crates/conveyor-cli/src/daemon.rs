//! Minimal local control-plane API over a repository-local Unix socket.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use conveyor_core::control_plane::ControlPlaneStore;

pub(crate) fn serve(directory: &Path) -> Result<(), String> {
    let socket = directory.join(".conveyor/conveyord.sock");
    if socket.exists() {
        return Err(format!(
            "socket {} already exists; stop the existing daemon before starting another",
            socket.display()
        ));
    }
    let listener = UnixListener::bind(&socket)
        .map_err(|error| format!("could not bind {}: {error}", socket.display()))?;
    eprintln!("conveyord: listening on {}", socket.display());
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let _ = handle(stream, directory);
            }
            Err(error) => eprintln!("conveyord: connection error: {error}"),
        }
    }
    Ok(())
}

fn handle(mut stream: UnixStream, directory: &Path) -> Result<(), String> {
    let mut request = String::new();
    BufReader::new(stream.try_clone().map_err(|error| error.to_string())?)
        .read_line(&mut request)
        .map_err(|error| error.to_string())?;
    let response = respond(request.trim(), directory);
    stream
        .write_all(format!("{response}\n").as_bytes())
        .map_err(|error| error.to_string())
}

fn respond(request: &str, directory: &Path) -> String {
    let store = match ControlPlaneStore::open(directory) {
        Ok(store) => store,
        Err(error) => return format!("error {error}"),
    };
    match request.split_whitespace().collect::<Vec<_>>().as_slice() {
        ["ping"] => "ok pong".to_owned(),
        ["decision", "list"] => match store.pending_decisions() {
            Ok(decisions) => decisions
                .into_iter()
                .map(|decision| format!("{}:{}:{}", decision.id, decision.kind, decision.summary))
                .collect::<Vec<_>>()
                .join("|"),
            Err(error) => format!("error {error}"),
        },
        [
            "decision",
            "answer",
            id,
            outcome @ ("allow_once" | "denied" | "cancelled"),
        ] => match id.parse::<i64>() {
            Ok(id) => match store.resolve_decision(id, outcome, now()) {
                Ok(true) => "ok".to_owned(),
                Ok(false) => "error not-pending".to_owned(),
                Err(error) => format!("error {error}"),
            },
            Err(_) => "error invalid-id".to_owned(),
        },
        _ => "error unsupported-request".to_owned(),
    }
}

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

#[cfg(test)]
mod tests {
    use super::respond;

    #[test]
    fn ping_is_a_local_health_check() {
        let directory = std::env::temp_dir().join(format!("conveyord-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        assert_eq!(respond("ping", &directory), "ok pong");
        std::fs::remove_dir_all(directory).unwrap();
    }
}
