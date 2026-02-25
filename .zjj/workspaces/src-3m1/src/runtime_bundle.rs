use anyhow::{anyhow, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const LOCAL_BIN_DIR: &str = ".oya/bin";
const LOCAL_BIN_NAME: &str = "oya";
const LOCAL_LAUNCHER_NAME: &str = "oya-local";
const TOOL_SUBDIR: &str = "tools";
const RESTATE_TOOL_NAME: &str = "restate";
const OPENCODE_TOOL_NAME: &str = "opencode";
const MOON_TOOL_NAME: &str = "moon";

pub(crate) fn install_local_binary() -> Result<PathBuf> {
    let repo_root = std::env::current_dir().context("resolve repo root")?;
    let source = std::env::current_exe().context("resolve running oya binary")?;
    let bin_dir = repo_root.join(LOCAL_BIN_DIR);
    fs::create_dir_all(&bin_dir).context("create local .oya/bin directory")?;

    let tools_dir = bin_dir.join(TOOL_SUBDIR);
    fs::create_dir_all(&tools_dir).context("create local .oya/bin/tools directory")?;

    let oya_path = copy_binary(&source, &bin_dir.join(LOCAL_BIN_NAME))?;
    let restate_path = bundle_tool("OYA_RESTATE_BIN", RESTATE_TOOL_NAME, &tools_dir)?;
    let opencode_path = bundle_tool("OPENCODE_PATH", OPENCODE_TOOL_NAME, &tools_dir)?;
    let moon_path = bundle_tool("MOON_PATH", MOON_TOOL_NAME, &tools_dir)?;

    let launcher = bin_dir.join(LOCAL_LAUNCHER_NAME);
    write_launcher_script(&launcher, &oya_path, &restate_path, &opencode_path, &moon_path)?;
    Ok(launcher)
}

fn copy_binary(source: &Path, destination: &Path) -> Result<PathBuf> {
    let temp_destination = destination.with_extension("tmp");
    fs::copy(source, &temp_destination).with_context(|| {
        format!("copy binary from {} to {}", source.display(), destination.display())
    })?;
    set_executable_permissions(&temp_destination)?;
    fs::rename(&temp_destination, destination).with_context(|| {
        format!(
            "move temporary binary from {} to {}",
            temp_destination.display(),
            destination.display()
        )
    })?;
    Ok(destination.to_path_buf())
}

fn bundle_tool(env_var: &str, binary_name: &str, tools_dir: &Path) -> Result<PathBuf> {
    let source = resolve_tool_binary(env_var, binary_name)?;
    copy_binary(&source, &tools_dir.join(binary_name))
}

fn resolve_tool_binary(env_var: &str, binary_name: &str) -> Result<PathBuf> {
    let from_env = std::env::var(env_var).ok().map(PathBuf::from);
    let from_path = lookup_binary(binary_name);
    let fallback = fallback_tool_path(binary_name);

    from_env
        .into_iter()
        .chain(from_path)
        .chain(fallback)
        .find(|candidate| candidate.exists())
        .ok_or_else(|| {
            anyhow!("{} binary not found. Set {} or install {}", binary_name, env_var, binary_name)
        })
}

fn fallback_tool_path(binary_name: &str) -> Option<PathBuf> {
    let home = std::env::var("HOME").ok()?;
    let path = match binary_name {
        RESTATE_TOOL_NAME => PathBuf::from(&home)
            .join(".local/share/mise/installs/ubi-restatedev-restate/latest/restate"),
        OPENCODE_TOOL_NAME => PathBuf::from(&home)
            .join(".local/share/mise/installs/github-sst-opencode/latest/opencode"),
        MOON_TOOL_NAME => PathBuf::from(&home).join(".local/bin/moon"),
        _ => return None,
    };
    Some(path)
}

fn lookup_binary(name: &str) -> Option<PathBuf> {
    let output = Command::new("which").arg(name).output().ok()?;
    if !output.status.success() {
        return None;
    }

    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (!value.is_empty()).then(|| PathBuf::from(value))
}

fn write_launcher_script(
    launcher_path: &Path,
    oya_path: &Path,
    restate_path: &Path,
    opencode_path: &Path,
    moon_path: &Path,
) -> Result<()> {
    let script = format!(
        "#!/usr/bin/env bash\nset -euo pipefail\nexport OYA_RESTATE_BIN=\"{}\"\nexport OPENCODE_PATH=\"{}\"\nexport MOON_PATH=\"{}\"\nexec \"{}\" \"$@\"\n",
        restate_path.display(),
        opencode_path.display(),
        moon_path.display(),
        oya_path.display()
    );

    fs::write(launcher_path, script)
        .with_context(|| format!("write launcher script {}", launcher_path.display()))?;
    set_executable_permissions(launcher_path)
}

#[cfg(unix)]
fn set_executable_permissions(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = fs::metadata(path)
        .with_context(|| format!("read metadata for {}", path.display()))?
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions)
        .with_context(|| format!("set executable permissions on {}", path.display()))
}

#[cfg(not(unix))]
fn set_executable_permissions(_path: &Path) -> Result<()> {
    Ok(())
}
