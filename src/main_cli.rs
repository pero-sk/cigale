use std::process::Command;
use std::path::PathBuf;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    let exe = std::env::current_exe().unwrap_or_default();
    let bin_dir = exe.parent().unwrap_or(std::path::Path::new("."));
    let pending = bin_dir.join(if cfg!(windows) { "cigale_pending.exe" } else { "cigale_pending" });
    if pending.exists() {
        let _ = std::fs::remove_file(&pending);
    }

    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(|s| s.as_str()) {
        Some("run")     => cmd_run(&args),
        Some("new")     => cmd_new(&args),
        Some("install") => cmd_install(&args),
        Some("fetch") => cmd_fetch(&args),
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
    println!("  cigale run                     -- run project using build.yml");
    println!("  cigale run <file.cig>          -- run a single file");
    println!("  cigale run <file.cig> [--no-stdl] -- run without stdl");
    println!("  cigale new <project_name>      -- create a new project");
    println!("  cigale install [version]       -- install cigale");
    println!("  cigale fetch [--global]        -- fetch dependencies from cigale.properties");
    println!("  cigale update                  -- update to latest");
    println!("  cigale version                 -- show version");
    println!("  cigale help                    -- show this help");
}

fn cmd_version() {
    println!("Cigale {}", VERSION);
}

fn cmd_run(args: &[String]) {
    // no file specified -- look for build.yml
    if args.get(2).is_none() || args.get(2).map(|s| s.starts_with("--")).unwrap_or(false) {
        let build_yml = std::path::Path::new("build.yml");
        if build_yml.exists() {
            run_from_build_yml(build_yml);
            return;
        } else {
            eprintln!("error: no file specified and no build.yml found in current directory");
            eprintln!("usage: cigale run <file.cig>");
            eprintln!("       cigale run  (if build.yml exists in cwd)");
            std::process::exit(1);
        }
    }

    let file = args.get(2).unwrap().clone();
    let no_stdl = args.iter().any(|a| a == "--no-stdl");
    run_file(&file, !no_stdl);
}

fn run_from_build_yml(path: &std::path::Path) {
    let content = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error reading build.yml: {}", e);
            std::process::exit(1);
        }
    };

    let mut entry = None;
    let mut use_stdl = true;

    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("entry:") {
            entry = Some(line.trim_start_matches("entry:").trim().to_string());
        } else if line.starts_with("stdl:") {
            let val = line.trim_start_matches("stdl:").trim();
            use_stdl = val == "true";
        }
    }

    let entry = match entry {
        Some(e) => e,
        None => {
            eprintln!("error: build.yml missing 'entry' field");
            std::process::exit(1);
        }
    };

    if !std::path::Path::new(&entry).exists() {
        eprintln!("error: entry point '{}' not found", entry);
        std::process::exit(1);
    }

    println!("Running {}...", entry);
    run_file(&entry, use_stdl);
}

fn run_file(file: &str, use_stdl: bool) {
    let bin_name = if !use_stdl {
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
        .arg(file)
        .status()
        .unwrap_or_else(|e| {
            eprintln!("failed to run {}: {}", bin_name, e);
            std::process::exit(1);
        });

    std::process::exit(status.code().unwrap_or(1));
}

fn cmd_fetch(args: &[String]) {
    let global = args.iter().any(|a| a == "--global");

    // find cigale.properties
    let props_path = std::path::Path::new("cigale.properties");
    if !props_path.exists() {
        eprintln!("error: no cigale.properties found in current directory");
        std::process::exit(1);
    }

    let content = match std::fs::read_to_string(props_path) {
        Ok(s) => s,
        Err(e) => { eprintln!("error reading cigale.properties: {}", e); std::process::exit(1); }
    };

    // parse dependencies
    let deps = parse_properties(&content);
    if deps.is_empty() {
        println!("No dependencies to fetch.");
        return;
    }

    // determine deps directory
    let deps_dir = if global {
        let home = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".cigale").join("packages")
    } else {
        PathBuf::from("deps")
    };

    std::fs::create_dir_all(&deps_dir).unwrap_or_else(|e| {
        eprintln!("error creating deps directory: {}", e);
        std::process::exit(1);
    });

    println!("Fetching {} dependenc{}...", deps.len(), if deps.len() == 1 { "y" } else { "ies" });

    for (name, url) in &deps {
        let dep_dir = deps_dir.join(name);
        if dep_dir.exists() {
            println!("  [OK] {} already fetched, pulling latest...", name);
            let status = Command::new("git")
                .args(&["pull"])
                .current_dir(&dep_dir)
                .status();
            match status {
                Ok(s) if s.success() => println!("  [OK] {} updated", name),
                _ => eprintln!("  [WARN] failed to update {}", name),
            }
        } else {
            println!("  Fetching {}...", name);
            let status = Command::new("git")
                .args(&["clone", url, dep_dir.to_str().unwrap()])
                .status();
            match status {
                Ok(s) if s.success() => println!("  [OK] {} fetched", name),
                _ => {
                    eprintln!("  [ERROR] failed to fetch {} from {}", name, url);
                    std::process::exit(1);
                }
            }
        }
    }

    println!("");
    println!("[OK] All dependencies fetched to {}", deps_dir.display());
}

fn parse_properties(content: &str) -> Vec<(String, String)> {
    let mut deps = Vec::new();
    let mut in_deps = false;

    for line in content.lines() {
        let line = line.trim();
        if line == "[dependencies]" {
            in_deps = true;
            continue;
        }
        if line.starts_with('[') {
            in_deps = false;
            continue;
        }
        if in_deps && !line.is_empty() && !line.starts_with('#') {
            // parse: name = "url"
            if let Some((name, url)) = line.split_once('=') {
                let name = name.trim().to_string();
                let url = url.trim().trim_matches('"').to_string();
                deps.push((name, url));
            }
        }
    }
    deps
}

fn cmd_new(args: &[String]) {
    let name = match args.get(2) {
        Some(n) => n.clone(),
        None => {
            eprintln!("usage: cigale new <project_name>");
            std::process::exit(1);
        }
    };

    let project_dir = std::path::Path::new(&name);
    if project_dir.exists() {
        eprintln!("error: directory '{}' already exists", name);
        std::process::exit(1);
    }

    // create project structure
    std::fs::create_dir_all(project_dir.join("src")).unwrap_or_else(|e| {
        eprintln!("error creating project directory: {}", e);
        std::process::exit(1);
    });

    // project.cfg
    std::fs::write(
        project_dir.join("project.cfg"),
        format!("name = \"{}\";\ndescription = \"\";\nversion = \"0.1.0\";\n", name)
    ).unwrap();

    // cigale.properties
    std::fs::write(
        project_dir.join("cigale.properties"),
        "[dependencies]\n"
    ).unwrap();

    // build.yml
    std::fs::write(
        project_dir.join("build.yml"),
        "entry: src/main.cig\nstdl: true\n"
    ).unwrap();

    // src/main.cig
    std::fs::write(
        project_dir.join("src").join("main.cig"),
        "import stdl.console { cout };\n\nfunc public static main() {\n    cout(\"Hello, world!\");\n}\n"
    ).unwrap();

    println!("Created project '{}'", name);
    println!("");
    println!("  {}/", name);
    println!("  ├── project.cfg");
    println!("  ├── cigale.properties");
    println!("  ├── build.yml");
    println!("  └── src/");
    println!("      └── main.cig");
    println!("");
    println!("  cd {} && cigale run", name);
}

fn cmd_install(args: &[String]) {
    let version = args.get(2).cloned();
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
    let script = if get_script_path("cigale_update.bat").exists() {
        get_script_path("cigale_update.bat")
    } else if get_script_path("cigale.bat").exists() {
        get_script_path("cigale.bat")
    } else {
        eprintln!("error: cigale_update.bat not found");
        eprintln!("try downloading cigale.bat and running it directly");
        std::process::exit(1);
    };
    let mut cmd = Command::new("cmd");
    cmd.args(&["/C", script.to_str().unwrap(), command]);
    if let Some(extra) = extra { cmd.arg(extra); }
    let status = cmd.status().unwrap_or_else(|e| {
        eprintln!("failed to run script: {}", e);
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
    let next_to_bin = get_bin_dir().join(name);
    if next_to_bin.exists() { return next_to_bin; }
    PathBuf::from(name)
}