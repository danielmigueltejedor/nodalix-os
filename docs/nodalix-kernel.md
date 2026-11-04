# Nodalix kernel

`linux-nodalix` is the Nodalix-managed kernel for 0.5.0. It uses a pinned
CachyOS source and generic x86-64 configuration. Its own package name and
kernel release suffix keep it separate from the Arch recovery kernel and
CachyOS packages. Ordinary Nodalix system updates skip `linux-nodalix`.
Kernel updates are included in the signed Nodalix release transaction.

## Build and test

Run `tools/build-nodalix-kernel.sh` in an up-to-date Arch build environment.
Use a build directory without spaces when building the Rust drivers. The
recipe pins the CachyOS source and configuration, verifies checksums, and
applies the same reviewed xpadneo patch used by the recovery kernel package.
The build keeps the per-build kernel module private key in the build tree.
Publish only the resulting package and its verified release metadata.

The build enables integrity lockdown and links xpadneo into the kernel.
Its native identifier setting applies to Microsoft BLE `045e:0b20`.
The package includes signed in-tree kernel modules. Existing fallback
kernels continue to use the separate xpadneo DKMS package where supported.

Release CI builds the kernel in a separate job. Local release builds take
the resulting archive via `NODALIX_KERNEL_ASSET` and verify its name and
version before adding it to the manifest.

## Release signing and rollout

1. Provision the project Minisign key. Commit only its public key at
   `updater/release.pub`. Deliver that key in a compatible stable updater
   before publishing the first signed kernel release so existing clients
   can verify the new manifest.
2. Provision the private key as `NODALIX_RELEASE_MINISIGN_KEY` in the
   trusted publishing environment. The unattended signer needs a key
   it can read without an interactive password. Keep it out of source
   control, build caches and the release assets directory.
3. Verify boot, a historical upgrade and a clean install. Keep the Arch
   recovery entry available. Test Secure Boot on a machine with enrolled
   keys and xpadneo on the supported controller before making the new
   kernel the default on that machine. A compatible clean install stages
   a one-time systemd-boot trial while keeping the recovery default. On a systemd-boot installation
   with the ESP at `/boot`, use `nodalix-kernel-boot stage --from-entry
   RECOVERY.conf` as root. After booting the trial, use
   `nodalix-kernel-boot promote`. Promotion requires both Linux kernel
   checks from fwupd to pass.

The release signature authenticates the package manifest. Secure Boot
checks the separate EFI signature against the keys enrolled on each PC.
On installations using sbctl, register the boot kernel with the machine's
enrolled owner key so the post-install hook signs updated images. A new
PC must trust the relevant public certificate before booting a signed EFI
image. Kernel lockdown also restricts external drivers. A machine relying
on an incompatible out-of-tree driver can use its Arch recovery entry.

[Upstream CachyOS kernel](https://github.com/CachyOS/linux-cachyos)
· [Linux module signing](https://docs.kernel.org/admin-guide/module-signing.html)
· [Secure Boot signing with sbctl](https://man.archlinux.org/man/sbctl.8.en)
