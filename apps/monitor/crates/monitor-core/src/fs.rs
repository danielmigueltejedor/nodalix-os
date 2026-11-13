//! Mounted file systems.

use std::ffi::CString;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mount {
    pub device: String,
    pub mount_point: String,
    pub fs_type: String,
    pub total: u64,
    pub free: u64,
}

/// Beyond this a file system is reporting no real limit (a FUSE mount without a quota says it
/// has exabytes), and showing the figure would only mislead.
const UNBOUNDED: u64 = 1 << 50;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// A disk or partition of this computer.
    Disk,
    /// A USB stick, card or other medium that comes and goes.
    Removable,
    /// Another computer's or a service's files.
    Network,
}

impl Mount {
    pub fn kind(&self) -> Kind {
        let network = self.fs_type.starts_with("nfs")
            || self.fs_type.starts_with("cifs")
            || self.fs_type.starts_with("smb")
            || self.fs_type.starts_with("fuse") && !self.device.starts_with('/');
        if network {
            Kind::Network
        } else if self.mount_point.starts_with("/run/media/")
            || self.mount_point.starts_with("/media/")
        {
            Kind::Removable
        } else {
            Kind::Disk
        }
    }

    /// Whether the size is a real one worth a bar.
    pub fn has_size(&self) -> bool {
        self.total < UNBOUNDED
    }

    pub fn used(&self) -> u64 {
        self.total.saturating_sub(self.free)
    }
    pub fn fraction(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            self.used() as f64 / self.total as f64
        }
    }
}

/// Kernel pseudo file systems that no one wants in a disk list.
const PSEUDO: &[&str] = &[
    "proc",
    "sysfs",
    "devtmpfs",
    "devpts",
    "tmpfs",
    "cgroup",
    "cgroup2",
    "pstore",
    "bpf",
    "securityfs",
    "debugfs",
    "tracefs",
    "configfs",
    "fusectl",
    "mqueue",
    "hugetlbfs",
    "autofs",
    "binfmt_misc",
    "efivarfs",
    "ramfs",
    "overlay",
    "squashfs",
    "nsfs",
    "fuse.portal",
    "fuse.gvfsd-fuse",
];

/// `/proc/self/mounts` escapes spaces and tabs as octal (`\040`).
fn unescape(field: &str) -> String {
    let mut out = String::with_capacity(field.len());
    let bytes = field.as_bytes();
    let mut i = 0;
    let mut raw = Vec::with_capacity(bytes.len());
    while i < bytes.len() {
        if bytes[i] == b'\\'
            && i + 3 < bytes.len()
            && bytes[i + 1..i + 4]
                .iter()
                .all(|b| (b'0'..=b'7').contains(b))
        {
            let v = (bytes[i + 1] - b'0') * 64 + (bytes[i + 2] - b'0') * 8 + (bytes[i + 3] - b'0');
            raw.push(v);
            i += 4;
        } else {
            raw.push(bytes[i]);
            i += 1;
        }
    }
    out.push_str(&String::from_utf8_lossy(&raw));
    out
}

/// Real mounts out of the mount table: `(device, mount point, type)`.
pub fn parse_mounts(text: &str) -> Vec<(String, String, String)> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for line in text.lines() {
        let mut f = line.split_whitespace();
        let (Some(dev), Some(mp), Some(ty)) = (f.next(), f.next(), f.next()) else {
            continue;
        };
        if PSEUDO.contains(&ty)
            || !dev.starts_with('/')
                && !ty.starts_with("nfs")
                && !ty.starts_with("cifs")
                && !ty.starts_with("fuse")
        {
            continue;
        }
        let (dev, mp) = (unescape(dev), unescape(mp));
        // A btrfs volume mounted at several subvolume paths is one disk to the user.
        if !seen.insert((dev.clone(), ty.to_string())) {
            continue;
        }
        out.push((dev, mp, ty.to_string()));
    }
    out
}

/// Total and free bytes of the file system holding `path`. Free counts every free block, the
/// ones reserved for root included, so that total minus free is what is really in use.
pub fn usage(path: &str) -> Option<(u64, u64)> {
    let c = CString::new(path).ok()?;
    let mut s: libc::statvfs = unsafe { std::mem::zeroed() };
    // SAFETY: `c` is a valid NUL-terminated path and `s` is a valid out pointer.
    if unsafe { libc::statvfs(c.as_ptr(), &mut s) } != 0 {
        return None;
    }
    let block = s.f_frsize as u64;
    Some((s.f_blocks as u64 * block, s.f_bfree as u64 * block))
}

pub fn read() -> Vec<Mount> {
    let Ok(text) = std::fs::read_to_string("/proc/self/mounts") else {
        return Vec::new();
    };
    parse_mounts(&text)
        .into_iter()
        .filter_map(|(device, mount_point, fs_type)| {
            let (total, free) = usage(&mount_point)?;
            (total > 0).then_some(Mount {
                device,
                mount_point,
                fs_type,
                total,
                free,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const MOUNTS: &str = "/dev/nvme0n1p2 / btrfs rw 0 0\nproc /proc proc rw 0 0\ntmpfs /run tmpfs rw 0 0\n/dev/nvme0n1p2 /home btrfs rw,subvol=/@home 0 0\n/dev/sda1 /mnt/My\\040Disk ext4 rw 0 0\nserver:/x /srv nfs4 rw 0 0\n";

    #[test]
    fn keeps_real_disks_and_drops_pseudo_ones() {
        let m = parse_mounts(MOUNTS);
        let points: Vec<&str> = m.iter().map(|(_, mp, _)| mp.as_str()).collect();
        assert_eq!(points, vec!["/", "/mnt/My Disk", "/srv"]);
    }

    #[test]
    fn unescapes_octal_sequences() {
        assert_eq!(unescape("a\\040b\\011c"), "a b\tc");
        assert_eq!(unescape("plain"), "plain");
        assert_eq!(unescape("trail\\04"), "trail\\04");
    }

    fn mount(device: &str, mount_point: &str, fs_type: &str, total: u64) -> Mount {
        Mount {
            device: device.into(),
            mount_point: mount_point.into(),
            fs_type: fs_type.into(),
            total,
            free: total,
        }
    }

    #[test]
    fn a_mount_is_a_disk_a_medium_or_someone_elses_files() {
        assert_eq!(mount("/dev/sda1", "/", "ext4", 1).kind(), Kind::Disk);
        assert_eq!(
            mount("/dev/sdb1", "/run/media/u/STICK", "vfat", 1).kind(),
            Kind::Removable
        );
        assert_eq!(mount("srv:/x", "/srv", "nfs4", 1).kind(), Kind::Network);
        assert_eq!(
            mount("nodalix-drive:", "/home/u/Nube/Drive", "fuse.rclone", 1).kind(),
            Kind::Network
        );
    }

    #[test]
    fn a_size_of_exabytes_is_not_a_size() {
        assert!(mount("/dev/sda1", "/", "ext4", 2 << 40).has_size());
        assert!(!mount("x:", "/m", "fuse.rclone", 1 << 60).has_size());
    }

    #[test]
    fn used_and_fraction() {
        let m = Mount {
            device: "d".into(),
            mount_point: "/".into(),
            fs_type: "ext4".into(),
            total: 200,
            free: 50,
        };
        assert_eq!(m.used(), 150);
        assert!((m.fraction() - 0.75).abs() < 1e-9);
        let empty = Mount {
            total: 0,
            free: 0,
            ..m
        };
        assert_eq!(empty.fraction(), 0.0);
    }

    #[test]
    fn reads_the_root_file_system_of_this_machine() {
        let mounts = read();
        assert!(mounts.iter().any(|m| m.total > 0));
    }
}
