//! Runs before K3s, so kubelet and containerd inherit the quota filesystem mount.
use anyhow::{Context, Result, ensure};
use std::{fs, path::Path, process::Command};

fn output(command: &mut Command) -> Result<String> {
    let result = command.output()?;
    ensure!(
        result.status.success(),
        "local node prerequisite failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    Ok(String::from_utf8(result.stdout)?)
}

fn main() -> Result<()> {
    let hostname = fs::read_to_string("/etc/hostname")?;
    ensure!(
        hostname.starts_with("k3d-devcenter-"),
        "this prerequisite is restricted to the dedicated local node"
    );
    // This private run-directory bind survives node replacement, like the shared kernel profile.
    let profiles = Path::new("/var/lib/kubelet/seccomp/substrate");
    ensure!(
        fs::symlink_metadata(profiles)?.is_dir(),
        "local profile receipt directory missing"
    );
    output(Command::new("/bin/chown").args(["0:0"]).arg(profiles))?;
    output(Command::new("/bin/chmod").args(["755"]).arg(profiles))?;
    let root = Path::new("/var/lib/devcenter-local");
    let image = root.join("workspaces.img");
    let mount = root.join("workspaces");
    fs::create_dir_all(&mount)?;
    if !image.exists() {
        let file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&image)?;
        file.set_len(4 * 1024 * 1024 * 1024)?;
        output(
            Command::new("/sbin/mkfs.ext4")
                .args(["-O", "quota,project", "-E", "quotatype=prjquota"])
                .arg(&image),
        )?;
    }
    ensure!(
        fs::symlink_metadata(&image)?.is_file(),
        "workspace backing must be a regular owned image"
    );
    let mounts = fs::read_to_string("/proc/mounts")?;
    if mounts
        .lines()
        .any(|line| line.split_whitespace().nth(1) == mount.to_str())
    {
        ensure!(
            mounts
                .lines()
                .any(|line| line.split_whitespace().nth(1) == mount.to_str()
                    && line.contains(" ext4 ")
                    && line.contains("prjquota")),
            "existing workspace mount has a different storage contract"
        );
        return Ok(());
    }
    let backing = format!("({})", image.display());
    let mut devices = output(Command::new("/bin/losetup").arg("-a"))?;
    if !devices.lines().any(|line| line.ends_with(&backing)) {
        output(Command::new("/bin/losetup").arg("-f").arg(&image))?;
        devices = output(Command::new("/bin/losetup").arg("-a"))?;
    }
    let matching = devices
        .lines()
        .filter(|line| line.ends_with(&backing))
        .collect::<Vec<_>>();
    ensure!(
        matching.len() == 1,
        "workspace image must have exactly one loop association"
    );
    let device = matching[0]
        .split(':')
        .next()
        .context("loop device missing")?;
    ensure!(device.starts_with("/dev/loop"), "unexpected loop device");
    output(
        Command::new("/bin/aux/mount")
            .args(["-t", "ext4", "-o", "prjquota", device])
            .arg(&mount),
    )?;
    // LOOP_CLR_FD marks a mounted loop device for automatic deletion on its final unmount.
    output(Command::new("/bin/losetup").args(["-d", device]))?;
    println!("local quota filesystem mounted before Kubernetes starts");
    Ok(())
}
