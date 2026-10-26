use mlua::Lua;
use std::{
    io::IsTerminal,
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex, OnceLock,
    },
    time::{Duration, Instant},
};
use tauri::{AppHandle, Manager};

const TAG: &str = "[postcmd]";
const RUN_TIMEOUT: Duration = Duration::from_secs(30);
const SHELL_TIMEOUT: Duration = Duration::from_secs(5);
const MARK: &str = "__WALL_ENV__";
const SHELL_VARS: &[&str] = &["PATH", "LANG", "LC_ALL", "LC_CTYPE"];

// Bumped on every pick so a hook still queued behind a slower one is skipped.
static LATEST: AtomicU64 = AtomicU64::new(0);

pub fn shell_env() -> &'static [(String, String)] {
    static ENV: OnceLock<Vec<(String, String)>> = OnceLock::new();
    ENV.get_or_init(|| {
        // A terminal launch already inherits the shell's env; Finder/Dock launches get launchd's bare one.
        let mut env = if std::io::stdin().is_terminal() {
            Vec::new()
        } else {
            login_shell_env()
        };
        let has_locale = |k: &str| env.iter().any(|(n, _)| n == k) || std::env::var_os(k).is_some();
        if !["LANG", "LC_ALL", "LC_CTYPE"].iter().any(|k| has_locale(k)) {
            env.push(("LANG".into(), "en_US.UTF-8".into()));
        }
        log::debug!("{TAG} child env: {env:?}");
        env
    })
}

fn login_shell_env() -> Vec<(String, String)> {
    let Ok(shell) = std::env::var("SHELL") else {
        return Vec::new();
    };
    let script = format!("printf {MARK}; /usr/bin/env -0; printf {MARK}");
    let argv = [shell, "-ilc".into(), script];
    let out = match exec(&argv, &[], SHELL_TIMEOUT) {
        Ok(out) => out,
        Err(e) => {
            log::warn!("{TAG} could not read login shell env, using inherited: {e}");
            return Vec::new();
        }
    };
    let Some(block) = out.split(MARK).nth(1) else {
        log::warn!("{TAG} login shell printed no env block, using inherited");
        return Vec::new();
    };
    block
        .split('\0')
        .filter_map(|kv| kv.split_once('='))
        .filter(|(k, _)| SHELL_VARS.contains(k))
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

fn exec(argv: &[String], env: &[(String, String)], timeout: Duration) -> Result<String, String> {
    let (prog, args) = argv.split_first().ok_or("empty command")?;
    let mut expr = duct::cmd(prog, args)
        .stdin_null()
        .stdout_capture()
        .stderr_capture()
        .unchecked();
    for (k, v) in env {
        expr = expr.env(k, v);
    }

    let handle = expr.start().map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound => format!("`{prog}` not found in PATH"),
        _ => format!("failed to start `{prog}`: {e}"),
    })?;
    let out = match handle.wait_timeout(timeout) {
        Ok(Some(out)) => out,
        Ok(None) => {
            handle.kill().ok();
            return Err(format!("`{prog}` timed out after {timeout:?}"));
        }
        Err(e) => return Err(format!("`{prog}` failed: {e}")),
    };

    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(format!("`{prog}` exited with {}: {}", out.status, stderr.trim()));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

pub fn run(argv: &[String]) -> Result<String, String> {
    exec(argv, shell_env(), RUN_TIMEOUT)
}

pub fn register(lua: &Lua) -> mlua::Result<()> {
    let wall = lua.create_table()?;
    wall.set(
        "run",
        lua.create_function(|_, argv: Vec<String>| run(&argv).map_err(mlua::Error::runtime))?,
    )?;
    lua.globals().set("wall", wall)
}

// Runs off the IPC thread so a slow hook never blocks the UI.
pub fn spawn_post_command(app: AppHandle, path: String) {
    let id = LATEST.fetch_add(1, Ordering::SeqCst) + 1;
    std::thread::spawn(move || {
        let state = app.state::<Mutex<Lua>>();
        let lua = state.lock().unwrap();
        if LATEST.load(Ordering::SeqCst) != id {
            log::info!("{TAG} skipped \"{path}\" — a newer wallpaper was picked");
            return;
        }

        let func = match lua.globals().get::<Option<mlua::Function>>("_wall_post_command") {
            Ok(Some(f)) => f,
            Ok(None) => return,
            Err(e) => return log::error!("{TAG} error getting function: {e}"),
        };

        log::info!("{TAG} calling post_command(\"{path}\")");
        let start = Instant::now();
        match func.call::<()>(path.as_str()) {
            Ok(()) => log::info!("{TAG} done in {:.2?}", start.elapsed()),
            Err(e) => log::error!("{TAG} failed after {:.2?}: {e}", start.elapsed()),
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn argv(a: &[&str]) -> Vec<String> {
        a.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn returns_stdout() {
        assert_eq!(run(&argv(&["echo", "hi"])).unwrap(), "hi\n");
    }

    #[test]
    fn nonzero_exit_includes_stderr() {
        let err = run(&argv(&["sh", "-c", "echo boom >&2; exit 3"])).unwrap_err();
        assert!(err.contains("exit status: 3") && err.contains("boom"), "{err}");
    }

    #[test]
    fn missing_program() {
        let err = run(&argv(&["definitely-not-a-real-binary"])).unwrap_err();
        assert!(err.contains("not found in PATH"), "{err}");
    }

    #[test]
    fn times_out() {
        let err = exec(&argv(&["sleep", "5"]), &[], Duration::from_millis(100)).unwrap_err();
        assert!(err.contains("timed out"), "{err}");
    }

    #[test]
    fn lua_error_aborts_hook() {
        let lua = Lua::new();
        register(&lua).unwrap();
        let err = lua.load(r#"wall.run({ "false" }); error("unreachable")"#).exec().unwrap_err();
        assert!(err.to_string().contains("`false` exited"), "{err}");
    }
}
