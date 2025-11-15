use std::{env, fs, process::Command};
use vergen::EmitBuilder;

fn main() {
    // Generate build & cargo info; guard git metadata based on worktree presence
    let mut emit_builder = EmitBuilder::builder();
    emit_builder.all_build().all_cargo();

    let in_git = Command::new("git")
        .args(["rev-parse", "--is-inside-work-tree"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if in_git {
        let has_head = Command::new("git")
            .args(["rev-parse", "--verify", "HEAD"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        if has_head {
            let _ = emit_builder.all_git();
        }
    }

    emit_builder
        .emit()
        .expect("Unable to generate build information");

    // Compute derived build version based on base semver and recent changes
    let base_ver = env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "0.0.0".into());
    let (maj0, min0, pat0) = parse_semver(&base_ver);
    let maj = maj0;
    let mut min = min0;
    let mut pat = pat0; // mut needed for bumping values

    // Heuristic: if git available and HEAD has parent, detect significant changes
    let mut significant = false;
    if in_git {
        if let Ok(out) = Command::new("git")
            .args(["rev-parse", "--verify", "HEAD~1"])
            .output()
        {
            if out.status.success() {
                if let Ok(diff) = Command::new("git")
                    .args(["diff", "--name-only", "HEAD~1..HEAD"])
                    .output()
                {
                    if diff.status.success() {
                        let txt = String::from_utf8_lossy(&diff.stdout);
                        for line in txt.lines() {
                            let p = line.trim();
                            if p.starts_with("src/handlers/")
                                || p == "src/models.rs"
                                || p == "src/db_manager.rs"
                                || p.starts_with("../src/entities/")
                            {
                                significant = true;
                                break;
                            }
                        }
                    }
                }
            }
        }
    }

    // Bump patch every build; bump minor if significant
    pat = pat.saturating_add(1);
    if significant {
        min = min.saturating_add(1);
    }
    let derived_version = format!("{}.{}.{}", maj, min, pat);
    println!("cargo:rustc-env=APP_BUILD_VERSION={}", derived_version);

    // Pass along description from Cargo (Cargo sets CARGO_PKG_DESCRIPTION if present)
    if let Ok(desc) = env::var("CARGO_PKG_DESCRIPTION") {
        println!("cargo:rustc-env=APP_PKG_DESCRIPTION={}", desc);
    }

    // Compute platform-specific suggested binary basename
    let _target = env::var("TARGET").unwrap_or_default();
    let arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    let os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    let arch_label = match arch.as_str() {
        "x86_64" => "64",
        "aarch64" => "arm64",
        "arm" => "arm",
        _ => arch.as_str(),
    };

    let mut os_label = os.clone();
    let mut os_ver = String::new();

    if os == "linux" {
        // Try /etc/os-release
        if let Ok(content) = fs::read_to_string("/etc/os-release") {
            let mut id = String::new();
            let mut ver = String::new();
            for line in content.lines() {
                if let Some(rest) = line.strip_prefix("ID=") {
                    id = rest.trim_matches('"').to_string();
                } else if let Some(rest) = line.strip_prefix("VERSION_ID=") {
                    ver = rest.trim_matches('"').to_string();
                }
            }
            if !id.is_empty() {
                os_label = id;
            }
            os_ver = ver;
        }
    } else if os == "macos" || os == "darwin" {
        os_label = "darwin".into();
        // Try sw_vers
        if let Ok(out) = Command::new("sw_vers").arg("-productVersion").output() {
            if out.status.success() {
                os_ver = String::from_utf8_lossy(&out.stdout).trim().to_string();
            }
        }
    } else if os == "windows" {
        os_label = "windows".into();
        // Windows version is not reliably available here; default to 11 if unknown
        os_ver = "11".into();
    }

    let _sep_ver = if os_ver.is_empty() {
        String::new()
    } else {
        format!("{}_", os_ver.replace('.', "_"))
    };
    let mut base = format!(
        "{}{}__{}",
        os_label,
        if os_ver.is_empty() {
            String::new()
        } else {
            format!("_{}", os_ver.replace('.', "_"))
        },
        arch_label
    );
    if os_label == "windows" {
        base.push_str(".exe");
    }

    println!("cargo:rustc-env=APP_BIN_FILENAME={}", base);
}

fn parse_semver(s: &str) -> (u64, u64, u64) {
    let mut maj = 0;
    let mut min = 0;
    let mut pat = 0;
    let parts: Vec<&str> = s.split('.').collect();
    if !parts.is_empty() {
        maj = parts[0].parse().unwrap_or(0);
    }
    if parts.len() > 1 {
        min = parts[1].parse().unwrap_or(0);
    }
    if parts.len() > 2 {
        pat = parts[2].parse().unwrap_or(0);
    }
    (maj, min, pat)
}
