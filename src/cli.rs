//! Command-line interface definitions.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};

use crate::PROJECT_NAME;

/// FurrumX command-line interface.
#[derive(Debug, Parser)]
#[command(name = "furrumx", version, about = "Arrow-native data platform")]
pub struct Cli {
    /// Command to execute.
    #[command(subcommand)]
    command: Command,
}

impl Cli {
    /// Executes the selected command.
    #[must_use]
    pub fn execute(self) -> ExitCode {
        match self.command {
            Command::Doctor => {
                println!("{}", DoctorReport::collect().render());
                ExitCode::SUCCESS
            }
        }
    }
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Inspect the local Linux/WSL development environment.
    Doctor,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DoctorReport {
    operating_system: &'static str,
    architecture: &'static str,
    logical_cpus: usize,
    memory_bytes: Option<u64>,
    workspace: PathBuf,
    is_wsl: bool,
    is_windows_mount: bool,
    enabled_features: Vec<&'static str>,
}

impl DoctorReport {
    fn collect() -> Self {
        let kernel_text = read_text("/proc/sys/kernel/osrelease")
            .or_else(|| read_text("/proc/version"))
            .unwrap_or_default();
        let workspace = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

        Self {
            operating_system: env::consts::OS,
            architecture: env::consts::ARCH,
            logical_cpus: std::thread::available_parallelism().map_or(1, usize::from),
            memory_bytes: read_text("/proc/meminfo").and_then(|text| parse_mem_total(&text)),
            is_wsl: detect_wsl(&kernel_text),
            is_windows_mount: is_windows_mounted_workspace(&workspace),
            workspace,
            enabled_features: enabled_features(),
        }
    }

    fn render(&self) -> String {
        let environment = if self.is_wsl { "WSL" } else { "native" };
        let memory = self.memory_bytes.map_or_else(
            || "unknown".to_owned(),
            |bytes| format!("{} MiB", bytes / 1_048_576),
        );
        let features = if self.enabled_features.is_empty() {
            "none".to_owned()
        } else {
            self.enabled_features.join(",")
        };

        let mut lines = vec![
            format!("{PROJECT_NAME} doctor"),
            format!(
                "platform: {}-{} ({environment})",
                self.operating_system, self.architecture
            ),
            format!("logical_cpus: {}", self.logical_cpus),
            format!("memory: {memory}"),
            format!("workspace: {}", self.workspace.display()),
            format!("features: {features}"),
        ];

        if self.is_windows_mount {
            lines.push(
                "warning: workspace is under /mnt; use the WSL Linux filesystem for valid I/O benchmarks"
                    .to_owned(),
            );
        } else {
            lines.push("workspace_filesystem: suitable for functional development".to_owned());
        }

        lines.join("\n")
    }
}

fn read_text(path: &str) -> Option<String> {
    fs::read_to_string(path).ok()
}

fn detect_wsl(kernel_text: &str) -> bool {
    let normalized = kernel_text.to_ascii_lowercase();
    normalized.contains("microsoft") || normalized.contains("wsl")
}

fn is_windows_mounted_workspace(path: &Path) -> bool {
    path.components()
        .next()
        .is_some_and(|root| root.as_os_str() == "/")
        && path.starts_with("/mnt")
}

fn parse_mem_total(meminfo: &str) -> Option<u64> {
    let line = meminfo.lines().find(|line| line.starts_with("MemTotal:"))?;
    let kibibytes = line.split_whitespace().nth(1)?.parse::<u64>().ok()?;
    kibibytes.checked_mul(1_024)
}

fn enabled_features() -> Vec<&'static str> {
    let mut features = Vec::new();

    if cfg!(feature = "local") {
        features.push("local");
    }
    if cfg!(feature = "flight-sql") {
        features.push("flight-sql");
    }
    if cfg!(feature = "distributed") {
        features.push("distributed");
    }
    if cfg!(feature = "wasm") {
        features.push("wasm");
    }
    if cfg!(feature = "python") {
        features.push("python");
    }
    if cfg!(feature = "s3") {
        features.push("s3");
    }
    if cfg!(feature = "http-store") {
        features.push("http-store");
    }

    features
}

#[cfg(test)]
mod tests {
    use super::{detect_wsl, is_windows_mounted_workspace, parse_mem_total};
    use std::path::Path;

    #[test]
    fn detects_wsl_kernel_markers() {
        assert!(detect_wsl("6.6.87.2-microsoft-standard-WSL2"));
        assert!(!detect_wsl("6.8.0-generic"));
    }

    #[test]
    fn detects_windows_mounted_workspace() {
        assert!(is_windows_mounted_workspace(Path::new(
            "/mnt/c/src/furrumx"
        )));
        assert!(!is_windows_mounted_workspace(Path::new(
            "/home/user/src/furrumx"
        )));
    }

    #[test]
    fn parses_linux_memory_total() {
        assert_eq!(
            parse_mem_total("MemTotal:       8192 kB\n"),
            Some(8_388_608)
        );
        assert_eq!(parse_mem_total("MemFree:        1024 kB\n"), None);
    }
}
