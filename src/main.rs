use glob::Pattern;
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::Command;
use std::time::Instant;
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};

fn load_ignore_patterns(file_path: &Path) -> Vec<Pattern> {
    let mut patterns = Vec::new();
    if let Ok(file) = File::open(file_path) {
        let reader = BufReader::new(file);
        for line in reader.lines().map_while(Result::ok) {
            let trimmed = line.trim();
            if !trimmed.is_empty() && !trimmed.starts_with('#') {
                // Handle both files and directories properly
                let mut adjusted_patterns = vec![trimmed.to_string()];

                // If it's a directory pattern, add an additional pattern for all subdirectories and files
                if trimmed.ends_with('/') || !trimmed.contains('.') {
                    adjusted_patterns.push(format!("{}/**", trimmed.trim_end_matches('/')));
                }

                // Add each adjusted pattern to the list
                for pattern_str in adjusted_patterns {
                    if let Ok(pattern) = Pattern::new(&pattern_str) {
                        patterns.push(pattern);
                    }
                }
            }
        }
    } else {
        println!(
            "Warning: Could not open ignore file: {}",
            file_path.display()
        );
    }
    patterns
}

fn should_ignore(path: &Path, ignore_patterns: &[Pattern]) -> bool {
    let path_str = path.to_string_lossy().replace("\\", "/");
    for pattern in ignore_patterns {
        if pattern.matches(&path_str) {
            return true;
        }
    }
    false
}

#[tokio::main]
async fn main() -> notify::Result<()> {
    // Get the watch path from command line arguments
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: {} <watch_path> <command>", args[0]);
        std::process::exit(1);
    }
    let watch_path = Path::new(&args[1]);
    let command = &args[2];

    // Create a channel to receive events.
    let (tx, mut rx) = mpsc::channel(1);

    // Create the watcher, with asynchronous configuration.
    let mut watcher: RecommendedWatcher = Watcher::new(
        move |res| {
            // Send the event to the async channel.
            let _ = tx.blocking_send(res);
        },
        Config::default(),
    )?;

    // Paths to watch and ignore
    let ignore_file_path = watch_path.join(".gitignore");

    // Load ignore patterns from .gitignore
    let ignore_patterns = load_ignore_patterns(&ignore_file_path);

    let mut last_event_times: HashMap<String, Instant> = HashMap::new();
    let debounce_duration = Duration::from_millis(5000); // Adjust as needed

    // Add the directory or file to be watched.
    watcher.watch(watch_path, RecursiveMode::Recursive)?;

    while let Some(res) = rx.recv().await {
        match res {
            Ok(Event { paths, .. }) => {
                for path in paths {
                    let relative_path = match path.strip_prefix(watch_path) {
                        Ok(rel_path) => rel_path,
                        Err(_) => {
                            println!("could not parse relative path");
                            continue;
                        } // If the path cannot be made relative, skip it
                    };

                    // Check if the path should be ignored
                    if should_ignore(relative_path, &ignore_patterns) {
                        continue; // Skip the event if the file is ignored
                    }

                    let path_str = path.to_string_lossy().to_string();
                    let now = Instant::now();

                    // Check if we recently processed this path
                    if let Some(last_time) = last_event_times.get(&path_str) {
                        if now.duration_since(*last_time) < debounce_duration {
                            last_event_times.insert(path_str.clone(), now);
                            continue; // Skip event if it's within the debounce window
                        }
                    }

                    // Update the last event time
                    last_event_times.insert(path_str.clone(), now);

                    println!("Change detected: {:?}", path_str);

                    // Execute a bash command asynchronously
                    let output = Command::new("bash")
                        .arg("-c")
                        .arg(command)
                        .output()
                        .expect("Failed to execute command");

                    // Print the output of the command
                    println!("{}", String::from_utf8_lossy(&output.stdout));
                }
            }
            Err(e) => println!("watch error: {:?}", e),
        }

        sleep(Duration::from_secs(1)).await;
    }

    Ok(())
}
