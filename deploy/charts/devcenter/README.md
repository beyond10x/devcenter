# Devcenter chart

The opt-in `substrate.execution.enabled` profile composes the Substrate container execution
bootstrap. It requires an immutable runtime image providing that entrypoint, amd64 nodes,
finite CPU/memory limits and the matching enforced node security profiles installed beforehand.

Set `substrate.execution.seccompProfile` to the relative kubelet profile path, such as
`substrate/host-exec-v1-amd64.json`. The AppArmor name is `substrate-host-exec-v1`.
The application pod has no host paths or service-account token. Its PID-1 setup process drops
root identity and bootstrap capabilities before starting the daemon as UID/GID 65532. The
existing project-quota option selects the quota executable after that transition.

`substrate.execution.temporarySize` bounds a memory-backed temporary directory; account for it
within the container's memory ceiling. Durable state and the separate workspace PVC retain
their existing mounts. The ordinary startup profile remains available when execution is disabled.

The execution profile sets `HOME=/nonexistent` before the bootstrap drops privileges. This keeps
Git initialization from reading the container runtime's root home and requires no writable home.

Only admit a Workspace terminal profile after the runtime's observed confinement and PTY facts
have passed deployment tests. Workspace profiles use the existing component `configFiles` and
`WORKSPACE_TERMINAL_PROFILES_PATH` configuration. Node profile installation and private terminal
admission values belong to the downstream deployment.
