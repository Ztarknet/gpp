use anyhow::{Context, Result};
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// Execute a command with streaming stdout and stderr, while also capturing stderr
///
/// This function streams both stdout and stderr in real-time to the terminal,
/// while also capturing stderr for error handling and metrics parsing.
///
/// # Arguments
/// * `cmd` - The command to execute
/// * `command_name` - Name of the command for error messages
///
/// # Returns
/// Tuple of (elapsed_time, stderr_output)
pub fn execute_with_streaming_output(
    cmd: &mut Command,
    command_name: &str,
) -> Result<(Duration, String)> {
    let start_time = Instant::now();

    // Force colored output even when stderr is piped
    // This ensures we get ANSI color codes that we can stream to the terminal
    cmd.env("CLICOLOR_FORCE", "1")  // Generic color forcing
        .env("FORCE_COLOR", "1");     // Used by some tools
    
    // Spawn process with stdout inherited (streamed) and stderr piped (streamed + captured)
    cmd.stdout(Stdio::inherit()).stderr(Stdio::piped());

    let mut child = cmd
        .spawn()
        .context(format!("Failed to spawn {}", command_name))?;

    // Capture and stream stderr in real-time
    let stderr_output = Arc::new(Mutex::new(String::new()));
    let stderr_output_clone = stderr_output.clone();

    let stderr_handle = if let Some(stderr) = child.stderr.take() {
        Some(thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines() {
                if let Ok(line) = line {
                    // Print to stderr in real-time
                    eprintln!("{}", line);
                    // Also capture for later use
                    if let Ok(mut output) = stderr_output_clone.lock() {
                        output.push_str(&line);
                        output.push('\n');
                    }
                }
            }
        }))
    } else {
        None
    };

    // Wait for process to complete
    let status = child
        .wait()
        .context(format!("Failed to wait for {}", command_name))?;

    // Wait for stderr thread to finish
    if let Some(handle) = stderr_handle {
        handle.join().ok();
    }

    let elapsed = start_time.elapsed();

    // Extract captured stderr
    let captured_stderr = stderr_output
        .lock()
        .map(|s| s.clone())
        .unwrap_or_default();

    if !status.success() {
        return Err(anyhow::anyhow!(
            "{} failed with exit code: {}",
            command_name,
            status.code().unwrap_or(-1)
        ));
    }

    Ok((elapsed, captured_stderr))
}

