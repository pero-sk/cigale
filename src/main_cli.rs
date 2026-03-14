use std::process::Command;
use std::path::PathBuf;
use std::fs;

const REPO_URL: &str = "https://github.com/YOUR_USERNAME/cigale";

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(|s| s.as_str()) {
        Some("install") => run_script("install", &[]),
        Some("update")  => run_script("update", &[]),
        Some("run")     => {
            let file = match args.get(2) {
                Some(f) => f.clone(),
                None => {
                    eprintln!("usage: cigale run <file.cig>");
                    std::process::exit(1);
                }
            };
            run_script("run", &[&file]);
        }
        Some("help") | None => print_help(),
        Some(cmd) => {
            eprintln!("unknown command: {}", cmd);
            print_help();
        }
    }
}

fn print_help() {
    println!("Cigale CLI");
    println!("----------");
    println!("  cigale install        -- download, build and install cigale");
    println!("  cigale update         -- update cigale to latest version");
    println!("  cigale run <file.cig> -- run a cigale file");
}

fn run_script(command: &str, extra_args: &[&str]) {
    if cfg!(windows) {
        run_batch(command, extra_args);
    } else {
        run_bash(command, extra_args);
    }
}

fn run_bash(command: &str, extra_args: &[&str]) {
    // look for script next to the binary
    let script_path = get_script_path("cigale.sh");
    if !script_path.exists() {
        eprintln!("error: cigale.sh not found at {}", script_path.display());
        std::process::exit(1);
    }
    let mut cmd = Command::new("bash");
    cmd.arg(&script_path).arg(command);
    for arg in extra_args { cmd.arg(arg); }
    let status = cmd.status().unwrap_or_else(|e| {
        eprintln!("failed to run cigale.sh: {}", e);
        std::process::exit(1);
    });
    std::process::exit(status.code().unwrap_or(1));
}

fn run_batch(command: &str, extra_args: &[&str]) {
    let script_path = get_script_path("cigale.bat");
    if !script_path.exists() {
        eprintln!("error: cigale.bat not found at {}", script_path.display());
        std::process::exit(1);
    }
    let mut cmd = Command::new("cmd");
    cmd.args(&["/C", script_path.to_str().unwrap(), command]);
    for arg in extra_args { cmd.arg(arg); }
    let status = cmd.status().unwrap_or_else(|e| {
        eprintln!("failed to run cigale.bat: {}", e);
        std::process::exit(1);
    });
    std::process::exit(status.code().unwrap_or(1));
}

fn get_script_path(script_name: &str) -> PathBuf {
    // look next to the executable first
    let exe = std::env::current_exe().unwrap_or_default();
    let next_to_exe = exe.parent().unwrap_or(std::path::Path::new(".")).join(script_name);
    if next_to_exe.exists() {
        return next_to_exe;
    }
    // fall back to current directory
    PathBuf::from(script_name)
}