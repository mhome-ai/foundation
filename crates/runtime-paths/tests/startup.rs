use runtime_paths::{
    resolve_runtime_workdir, runtime_env, runtime_workdir, set_runtime_context,
    set_runtime_workdir, RuntimeEnv,
};
use std::path::Path;

#[test]
fn explicit_startup_is_independent_of_home_and_directory_names() {
    const CHILD: &str = "RUNTIME_PATHS_STARTUP_TEST";
    let Ok(mode) = std::env::var(CHILD) else {
        for mode in ["prod", "dev", "e2e", "embedded"] {
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "explicit_startup_is_independent_of_home_and_directory_names",
                    "--nocapture",
                ])
                .env(CHILD, mode)
                .env_remove("HOME")
                .env_remove("LOCALAPPDATA")
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{mode}: {} {}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
        return;
    };
    let home = std::env::temp_dir().join("explicit-runtime-home");
    assert!(resolve_runtime_workdir(None, RuntimeEnv::Prod).is_err());
    if mode == "embedded" {
        let dir = home.join("dev");
        set_runtime_workdir(dir.clone()).unwrap();
        assert_eq!(runtime_env(), RuntimeEnv::Prod);
        assert_eq!(runtime_workdir(), dir);
        return;
    }
    let env = RuntimeEnv::from_optional(Some(&mode)).unwrap();
    let expected = if env.is_prod() {
        home.clone()
    } else {
        home.join(&mode)
    };
    assert_eq!(resolve_runtime_workdir(Some(&home), env).unwrap(), expected);
    // Internal services inherit env explicitly even when their path has another name.
    let dir = home.join("custom/dev/e2e");
    set_runtime_context(dir.clone(), env).unwrap();
    assert_eq!(runtime_env(), env);
    assert_eq!(runtime_workdir(), dir);
    set_runtime_context(dir.clone(), env).unwrap();
    assert!(set_runtime_context(dir.join("other"), env).is_err());
    let other = if env.is_prod() {
        RuntimeEnv::Dev
    } else {
        RuntimeEnv::Prod
    };
    assert!(set_runtime_context(dir, other).is_err());
    assert!(resolve_runtime_workdir(Some(Path::new("relative")), env).is_err());
}

#[cfg(unix)]
#[test]
fn isolated_workdirs_cannot_redirect_to_production() {
    let root = std::env::temp_dir().join(format!("runtime-env-symlink-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    for env in [RuntimeEnv::Dev, RuntimeEnv::E2e] {
        let link = root.join(env.as_str());
        std::os::unix::fs::symlink(&root, &link).unwrap();
        assert!(resolve_runtime_workdir(Some(&root), env).is_err());
        std::fs::remove_file(link).unwrap();
    }
    std::fs::remove_dir(root).unwrap();
}
