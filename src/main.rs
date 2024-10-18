use glob::Pattern;
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::Command;
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};
use watch_and_run::utils::recent_strings::RecentStrings;

fn handle_args() -> (PathBuf, String) {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: {} <watch_path> <command>", args[0]);
        std::process::exit(1);
    }
    let mut watch_path = PathBuf::from(&args[1]);
    if watch_path.is_relative() {
        watch_path = watch_path.canonicalize().unwrap();
    }
    let command = args[2].clone();
    (watch_path, command)
}

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
    if let Ok(pattern) = Pattern::new(".git/**") {
        patterns.push(pattern);
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

fn calculate_sha256(file_path: &Path) -> Result<String, std::io::Error> {
    let file = File::open(file_path)?;
    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut buffer = [0; 1024];

    loop {
        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    let result = hasher.finalize();
    Ok(format!("{:x}", result))
}

#[tokio::main]
async fn main() -> notify::Result<()> {
    let (watch_path, command) = handle_args();

    // Create a channel to receive events.
    let (tx, mut rx) = mpsc::channel(200);

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

    let mut last_file_hash: HashMap<String, RecentStrings> = HashMap::new();

    // Add the directory or file to be watched.
    watcher.watch(&watch_path, RecursiveMode::Recursive)?;

    while let Some(res) = rx.recv().await {
        match res {
            Ok(Event { paths, .. }) => {
                for path in paths {
                    let relative_path = match path.strip_prefix(&watch_path) {
                        Ok(rel_path) => rel_path,
                        Err(_) => {
                            println!("could not parse relative path");
                            continue; // If the path cannot be made relative, skip it
                        }
                    };

                    if should_ignore(relative_path, &ignore_patterns) {
                        continue;
                    }

                    let path_str = path.to_string_lossy().to_string();
                    let current_hash = calculate_sha256(&path)?;

                    if let Some(last_hash) = last_file_hash.get_mut(&path_str) {
                        if last_hash.contains(&current_hash) {
                            // no changes to this file since the last time we saw it, skip forward
                            continue;
                        } else {
                            last_hash.add(current_hash);
                        }
                    } else {
                        let mut string_hash = RecentStrings::default();
                        string_hash.add(current_hash);
                        last_file_hash.insert(path_str.clone(), string_hash);
                    }

                    println!("Change detected: {:?}", path_str);

                    // Execute a bash command asynchronously
                    let output = Command::new("bash")
                        .arg("-c")
                        .arg(&command)
                        .output()
                        .expect("Failed to execute command");

                    // Print the output of the command
                    println!("{}", String::from_utf8_lossy(&output.stdout));
                    println!("{}", String::from_utf8_lossy(&output.stderr));
                    break;
                }
            }
            Err(e) => println!("watch error: {:?}", e),
        }
        if rx.is_empty() {
            sleep(Duration::from_secs(1)).await;
        }
    }

    Ok(())
}
