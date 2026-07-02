use std::path::Path;
use std::time::{Duration, Instant};

/// Wait for the daemon endpoint and PID file to disappear after shutdown.
pub async fn wait_until_stopped(project_root: &Path, timeout: Duration) -> bool {
    let endpoint = super::local_endpoint(project_root);
    let pid_file = super::pid_file_path(project_root);
    let started = Instant::now();

    loop {
        if !roam_local::endpoint_exists(&endpoint) && !pid_file.exists() {
            return true;
        }

        if started.elapsed() >= timeout {
            return false;
        }

        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}
