# mhome-runtime-paths

Cross-platform runtime directories, local endpoint names, process ownership checks, and stale-socket
handling shared by mHome local processes.

The crate contains local operating-system infrastructure only. It does not start services or own
application lifecycle policy.

Startup selects a local `env` (`prod` by default, `dev`, or `e2e`) before resolving storage.
`resolve_runtime_workdir(Some(home), env)` uses `home` for production and `home/dev`
or `home/e2e` otherwise. Omit the runtime home to use the platform's `.meow` root.
Call `set_runtime_context(workdir, env)` before starting a non-production process;
`set_runtime_workdir(workdir)` retains an explicit production default for embedded callers.
Neither initializer infers an environment from directory names or reads the OS user home.
