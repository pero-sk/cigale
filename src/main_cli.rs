use std::process::Command;
use std::path::PathBuf;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(|s| s.as_str()) {
        Some("run")     => cmd_run(&args),
        Some("install") => cmd_install(&args),
        Some("update")  => cmd_update(),
        Some("version") => cmd_version(),
        Some("help") | None => cmd_help(),
        Some(cmd) => {
            eprintln!("unknown command: {}", cmd);
            eprintln!("run 'cigale help' for usage");
            std::process::exit(1);
        }
    }
}

fn cmd_help() {
    println!("Cigale {}", VERSION);
    println!("");
    println!("usage:");
    println!("  cigale run <file.cig> [--no-stdl]  -- run a cigale file");
    println!("  cigale install [version]            -- install cigale");
    println!("  cigale update                       -- update to latest");
    println!("  cigale version                      -- show version");
    println!("  cigale help                         -- show this help");
}

fn cmd_version() {
    println!("Cigale {}", VERSION);
}

fn cmd_run(args: &[String]) {
    let file = match args.get(2) {
        Some(f) => f.clone(),
        None => {
            eprintln!("usage: cigale run <file.cig> [--no-stdl]");
            std::process::exit(1);
        }
    };

    let no_stdl = args.iter().any(|a| a == "--no-stdl");

    // find cigale_stdl or cigale_nostdl next to this binary
    let bin_name = if no_stdl {
        if cfg!(windows) { "cigale_nostdl.exe" } else { "cigale_nostdl" }
    } else {
        if cfg!(windows) { "cigale_stdl.exe" } else { "cigale_stdl" }
    };

    let bin_path = get_bin_dir().join(bin_name);

    if !bin_path.exists() {
        eprintln!("error: {} not found at {}", bin_name, bin_path.display());
        eprintln!("try running: cigale install");
        std::process::exit(1);
    }

    let status = Command::new(&bin_path)
        .arg(&file)
        .status()
        .unwrap_or_else(|e| {
            eprintln!("failed to run {}: {}", bin_name, e);
            std::process::exit(1);
        });

    std::process::exit(status.code().unwrap_or(1));
}

fn cmd_install(args: &[String]) {
    let version = args.get(2).cloned(); // optional version
    run_script("install", version.as_deref());
}

fn cmd_update() {
    run_script("update", None);
}

fn run_script(command: &str, extra: Option<&str>) {
    if cfg!(windows) {
        run_batch(command, extra);
    } else {
        run_bash(command, extra);
    }
}

fn run_bash(command: &str, extra: Option<&str>) {
    let script = get_script_path("cigale.sh");
    if !script.exists() {
        eprintln!("error: cigale.sh not found at {}", script.display());
        std::process::exit(1);
    }
    let mut cmd = Command::new("bash");
    cmd.arg(&script).arg(command);
    if let Some(extra) = extra { cmd.arg(extra); }
    let status = cmd.status().unwrap_or_else(|e| {
        eprintln!("failed to run cigale.sh: {}", e);
        std::process::exit(1);
    });
    std::process::exit(status.code().unwrap_or(1));
}

fn run_batch(command: &str, extra: Option<&str>) {
    let script = get_script_path("cigale.bat");
    if !script.exists() {
        eprintln!("error: cigale.bat not found at {}", script.display());
        std::process::exit(1);
    }
    let mut cmd = Command::new("cmd");
    cmd.args(&["/C", script.to_str().unwrap(), command]);
    if let Some(extra) = extra { cmd.arg(extra); }
    let status = cmd.status().unwrap_or_else(|e| {
        eprintln!("failed to run cigale.bat: {}", e);
        std::process::exit(1);
    });
    std::process::exit(status.code().unwrap_or(1));
}

fn get_bin_dir() -> PathBuf {
    std::env::current_exe()
        .unwrap_or_default()
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .to_path_buf()
}

fn get_script_path(name: &str) -> PathBuf {
    // next to binary first
    let next_to_bin = get_bin_dir().join(name);
    if next_to_bin.exists() { return next_to_bin; }
    // fall back to cwd
    PathBuf::from(name)
}