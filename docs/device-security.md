# Device Security in Nodalix

GNOME Settings → Privacy & Security → Device Security shows the Host Security
ID reported by fwupd. Use the detailed report or `fwupdmgr security` to see
which protections apply to this computer.

- UEFI Secure Boot, TPM 2.0 and IOMMU each have their own check. Check the
  computer manufacturer's firmware documentation for hardware-specific
  settings. AMD Platform Secure Boot is a separate firmware protection from
  UEFI Secure Boot. Memory encryption and SPI protections also depend on
  platform support.
- Kernel Lockdown is measured independently of UEFI Secure Boot. Before
  changing the boot configuration, check the trusted signing keys for all
  required modules, including xpadneo, and keep a working recovery boot entry.
  A signature embedded in a module only establishes trust if the running
  kernel recognizes its signing key.
- The kernel taint report can identify an external or untrusted module.
  `cat /proc/sys/kernel/tainted` reports the kernel's flags and
  `cat /sys/module/hid_xpadneo/taint` reports xpadneo's flags when it is
  loaded. Loading an external module still records that it was built outside
  the kernel tree.
- Suspend checks depend on the active sleep mode and firmware. Confirm that a
  supported mode resumes reliably on the device before changing the default.
  Software disk encryption and drive-managed hardware encryption are reported
  separately.

[fwupd Host Security ID](https://fwupd.github.io/libfwupdplugin/hsi.html)
· [Linux kernel taint flags](https://docs.kernel.org/admin-guide/tainted-kernels.html)
· [Nodalix Xbox controller support](xbox-controller.md)
