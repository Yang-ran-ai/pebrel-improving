//! DeepSeek Harness Desktop discovery and its independent Desktop-profile bridge.
//!
//! The desktop application owns its runtime. Pebrel only manages the local
//! Desktop profile bundle and launches the installed application; it never
//! mutates the user's global Node/npm installation.

use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const BUNDLE_NAME: &str = "@pebrel/deepseek-harness-bridge";
const DOWNLOAD_PAGE: &str = "https://github.com/deepseek-ai/deepseek-harness/releases/latest";

const BRIDGE_JS: &str =
    include_str!("../../../../integrations/deepseek-harness-bridge/index.js");
const BRIDGE_PACKAGE: &str =
    include_str!("../../../../integrations/deepseek-harness-bridge/package.json");
const BRIDGE_PATCH: &str =
    include_str!("../../../../integrations/deepseek-harness-bridge/cordis.patch.yml");

#[derive(Clone, Debug, Default)]
pub(crate) struct State {
    pub desktop_executable: Option<PathBuf>,
    pub desktop_installed: bool,
    pub desktop_running: bool,
    pub dsh_cli: Option<PathBuf>,
    pub profile_manifest: Option<PathBuf>,
    pub profile_initialized: bool,
    pub bridge_directory: PathBuf,
    pub hook_installed: bool,
    pub hook_current: bool,
}

pub(crate) fn inspect() -> State {
    let desktop_executable = desktop_executable();
    let dsh_cli = dsh_cli(desktop_executable.as_deref());
    let profile_manifest = profile_dir().map(|path| path.join("package.json"));
    let profile_initialized =
        profile_manifest.as_ref().is_some_and(|path| path.is_file());
    let bridge_directory = bridge_directory();
    let (hook_installed, hook_current) = profile_manifest
        .as_deref()
        .map(|path| inspect_manifest(path, &bridge_directory))
        .unwrap_or_default();

    State {
        desktop_installed: desktop_executable.is_some(),
        desktop_running: desktop_running(),
        desktop_executable,
        dsh_cli,
        profile_manifest,
        profile_initialized,
        bridge_directory,
        hook_installed,
        hook_current,
    }
}

pub(crate) fn profile_manifest() -> Option<PathBuf> {
    profile_dir().map(|path| path.join("package.json"))
}

fn profile_dir() -> Option<PathBuf> {
    let home = std::env::var_os("DSH_HOME")
        .map(PathBuf::from)
        .or_else(|| crate::platform::dirs::home_dir().map(|home| home.join(".dsh")))?;
    Some(home.join("profiles").join("desktop"))
}

fn bridge_directory() -> PathBuf {
    pebrel_settings::settings_dir()
        .join("integrations")
        .join("deepseek-harness-bridge")
}

pub(crate) fn desktop_executable() -> Option<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        let local = PathBuf::from(local);
        candidates.extend([
            local.join("Programs/DeepSeek Harness/DeepSeek Harness.exe"),
            local.join("DeepSeek Harness/DeepSeek Harness.exe"),
        ]);
    }
    for variable in ["ProgramFiles", "ProgramW6432"] {
        if let Some(program_files) = std::env::var_os(variable) {
            candidates.push(
                PathBuf::from(program_files).join("DeepSeek Harness/DeepSeek Harness.exe"),
            );
        }
    }
    if let Some(home) = crate::platform::dirs::home_dir() {
        candidates.push(home.join("Applications/DeepSeek Harness.app/Contents/MacOS/DeepSeek Harness"));
        candidates.push(home.join(".local/bin/deepseek-harness-desktop"));
    }
    candidates.into_iter().find(|path| path.is_file())
}

fn dsh_cli(desktop: Option<&Path>) -> Option<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(executable) = desktop {
        if let Some(root) = executable.parent() {
            candidates.extend([
                root.join("resources/runtime/cli/bin/dsh.cmd"),
                root.join("resources/runtime/cli/bin/dsh"),
                root.join("Contents/Resources/runtime/cli/bin/dsh"),
            ]);
        }
    }
    if let Some(path) = std::env::var_os("PATH") {
        for directory in std::env::split_paths(&path) {
            candidates.extend(["dsh.cmd", "dsh.exe", "dsh"].map(|name| directory.join(name)));
        }
    }
    candidates.into_iter().find(|path| path.is_file())
}

fn desktop_running() -> bool {
    #[cfg(windows)]
    {
        return Command::new("tasklist.exe")
            .args(["/FI", "IMAGENAME eq DeepSeek Harness.exe", "/FO", "CSV", "/NH"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success());
    }
    #[cfg(not(windows))]
    {
        Command::new("pgrep")
            .args(["-f", "DeepSeek Harness"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success())
    }
}

fn inspect_manifest(path: &Path, bridge_dir: &Path) -> (bool, bool) {
    let Ok(raw) = std::fs::read_to_string(path) else { return (false, false) };
    let Ok(manifest) = serde_json::from_str::<Value>(&raw) else { return (false, false) };
    let dependency = manifest
        .get("dependencies")
        .and_then(Value::as_object)
        .and_then(|dependencies| dependencies.get(BUNDLE_NAME))
        .is_some();
    let bundle = manifest
        .get("dsh")
        .and_then(|dsh| dsh.get("profile"))
        .and_then(|profile| profile.get("bundles"))
        .and_then(Value::as_array)
        .is_some_and(|bundles| bundles.iter().any(|value| value.as_str() == Some(BUNDLE_NAME)));
    let installed = dependency && bundle;
    let current = installed
        && std::fs::read_to_string(bridge_dir.join("index.js")).as_deref() == Ok(BRIDGE_JS)
        && std::fs::read_to_string(bridge_dir.join("package.json")).as_deref()
            == Ok(BRIDGE_PACKAGE)
        && std::fs::read_to_string(bridge_dir.join("cordis.patch.yml")).as_deref()
            == Ok(BRIDGE_PATCH);
    (installed, current)
}

fn write_owned_bundle() -> std::io::Result<()> {
    let directory = bridge_directory();
    std::fs::create_dir_all(&directory)?;
    for (name, content) in [
        ("index.js", BRIDGE_JS),
        ("package.json", BRIDGE_PACKAGE),
        ("cordis.patch.yml", BRIDGE_PATCH),
    ] {
        let path = directory.join(name);
        match std::fs::read_to_string(&path) {
            Ok(existing) if existing == content => continue,
            Ok(_) => {
                return Err(std::io::Error::other(format!(
                    "Pebrel bridge file was edited; preserving it: {}",
                    path.display()
                )));
            },
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {},
            Err(error) => return Err(error),
        }
        let temporary = path.with_extension(format!("pebrel-tmp-{}", std::process::id()));
        std::fs::write(&temporary, content)?;
        if let Err(error) = std::fs::rename(&temporary, &path) {
            let _ = std::fs::remove_file(&temporary);
            return Err(error);
        }
    }
    Ok(())
}

fn run_dsh(cli: &Path, arguments: &[&str]) -> std::io::Result<()> {
    #[cfg(windows)]
    {
        let extension_is_cmd = cli.extension().is_some_and(|extension| {
            extension.eq_ignore_ascii_case("cmd") || extension.eq_ignore_ascii_case("bat")
        });
        if extension_is_cmd {
            // Windows command scripts must be invoked through cmd.exe. The only
            // interpolated values are absolute filesystem paths, quoted as one argv.
            let mut command_line = format!("\"{}\"", cli.display());
            for argument in arguments {
                command_line.push(' ');
                command_line.push_str(&format!("\"{}\"", argument.replace('"', "\"\"")));
            }
            let status = Command::new("cmd.exe")
                .args(["/D", "/S", "/C"])
                .arg(command_line)
                .status()?;
            return if status.success() {
                Ok(())
            } else {
                Err(std::io::Error::other(format!("dsh exited with {status}")))
            };
        }
    }
    let status = Command::new(cli).args(arguments).status()?;
    if status.success() {
        Ok(())
    } else {
        Err(std::io::Error::other(format!("dsh exited with {status}")))
    }
}

pub(crate) fn install_hook() -> std::io::Result<()> {
    let state = inspect();
    if !state.desktop_installed {
        return Err(std::io::Error::other(
            "DeepSeek Harness Desktop is not installed. Install the desktop application first.",
        ));
    }
    if !state.profile_initialized {
        return Err(std::io::Error::other(
            "Open DeepSeek Harness Desktop once to initialize its desktop profile, then refresh Agents.",
        ));
    }
    let cli = state.dsh_cli.ok_or_else(|| {
        std::io::Error::other("The Desktop-bundled dsh command was not found.")
    })?;
    write_owned_bundle()?;
    let bundle = bridge_directory().to_string_lossy().into_owned();
    run_dsh(&cli, &["plugin", "--profile", "desktop", "add", &bundle])?;
    let state = inspect();
    if !state.hook_installed || !state.hook_current {
        return Err(std::io::Error::other(
            "dsh completed but the DeepSeek Desktop profile did not verify the Pebrel bridge.",
        ));
    }
    Ok(())
}

pub(crate) fn remove_hook() -> std::io::Result<()> {
    let state = inspect();
    if !state.hook_installed {
        return Ok(());
    }
    let cli = state.dsh_cli.ok_or_else(|| {
        std::io::Error::other("The Desktop-bundled dsh command was not found.")
    })?;
    run_dsh(&cli, &["plugin", "--profile", "desktop", "remove", BUNDLE_NAME])?;
    if inspect().hook_installed {
        return Err(std::io::Error::other("The DeepSeek Harness bridge remains in the Desktop profile."));
    }
    Ok(())
}

pub(crate) fn launch_desktop() -> std::io::Result<()> {
    let state = inspect();
    let executable = state.desktop_executable.ok_or_else(|| {
        std::io::Error::other("DeepSeek Harness Desktop is not installed.")
    })?;
    let mut command = Command::new(executable);
    if let Some(helper) = super::helper_path() {
        command.env(crate::ai_hook::HOOK_EXE_ENV, helper);
    }
    // The pipe is process-wide; when the desktop is launched from the Agents
    // surface the hook router safely falls back to the currently focused pane.
    if let Some(pipe) = std::env::var_os(crate::ai_hook::PIPE_ENV) {
        command.env(crate::ai_hook::PIPE_ENV, pipe);
    }
    command.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    command.spawn()?;
    Ok(())
}

pub(crate) fn uninstall_desktop() -> std::io::Result<()> {
    let state = inspect();
    let executable = state.desktop_executable.ok_or_else(|| {
        std::io::Error::other("DeepSeek Harness Desktop is not installed.")
    })?;
    let root = executable.parent().ok_or_else(|| {
        std::io::Error::other("Could not resolve the DeepSeek Harness installation directory.")
    })?;
    let uninstaller = [
        "Uninstall DeepSeek Harness.exe",
        "unins000.exe",
        "uninstall.exe",
    ]
    .into_iter()
    .map(|name| root.join(name))
    .find(|path| path.is_file())
    .ok_or_else(|| std::io::Error::other("The DeepSeek Harness uninstaller was not found."))?;
    Command::new(uninstaller).spawn()?;
    Ok(())
}

pub(crate) fn open_download_page() -> std::io::Result<()> {
    #[cfg(windows)]
    let mut command = {
        let mut command = Command::new("cmd.exe");
        command.args(["/C", "start", "", DOWNLOAD_PAGE]);
        command
    };
    #[cfg(target_os = "macos")]
    let mut command = Command::new("open");
    #[cfg(all(unix, not(target_os = "macos")))]
    let mut command = Command::new("xdg-open");
    #[cfg(unix)]
    command.arg(DOWNLOAD_PAGE);
    command.spawn().map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_bridge_presence_requires_both_dependency_and_bundle_layer() {
        let directory = tempfile::tempdir().unwrap();
        let bridge = directory.path().join("bridge");
        std::fs::create_dir_all(&bridge).unwrap();
        std::fs::write(bridge.join("index.js"), BRIDGE_JS).unwrap();
        std::fs::write(bridge.join("package.json"), BRIDGE_PACKAGE).unwrap();
        std::fs::write(bridge.join("cordis.patch.yml"), BRIDGE_PATCH).unwrap();
        let manifest = directory.path().join("package.json");
        let profile = serde_json::json!({
            "dependencies": { BUNDLE_NAME: "link:bridge" },
            "dsh": { "profile": { "bundles": ["@deepseek-ai/dsh-base", BUNDLE_NAME] } }
        });
        std::fs::write(&manifest, serde_json::to_vec(&profile).unwrap()).unwrap();
        assert_eq!(inspect_manifest(&manifest, &bridge), (true, true));

        let without_layer = serde_json::json!({
            "dependencies": { BUNDLE_NAME: "link:bridge" },
            "dsh": { "profile": { "bundles": ["@deepseek-ai/dsh-base"] } }
        });
        std::fs::write(&manifest, serde_json::to_vec(&without_layer).unwrap()).unwrap();
        assert_eq!(inspect_manifest(&manifest, &bridge), (false, false));
    }
}
