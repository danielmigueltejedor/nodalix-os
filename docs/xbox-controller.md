# Xbox controllers in Nodalix OS

Nodalix ships `nodalix-xpadneo-dkms`, a **versioned xpadneo package** for
supported Xbox Bluetooth controllers. It is not a replacement for BlueZ, the
kernel's USB `xpad` driver or the Xbox Wireless Adapter driver.

## Why the Nodalix package exists

Upstream xpadneo 0.10.5 intentionally advertises its supported controllers
as Microsoft `045e:028e` (Xbox 360) for older game compatibility. Nodalix
has verified that the Bluetooth LE device `045e:0b20` can retain its own
identity with xpadneo on the tested system.

The Nodalix patch introduces `disable_pid_spoofing`, but **only bypasses
spoofing for Microsoft `045e:0b20`**. All other devices keep upstream's
mapping even when the option is enabled. This is deliberately narrower than
the current upstream draft to reduce compatibility risk.

The upstream source is pinned to a reviewed Git commit, and the patch is
applied with `patch --fuzz=0`; an upstream source change that no longer
matches causes a build failure, not a silent loss of the fix.

## Updates and lifecycle

- The package is an official component of a Nodalix release. It
  `provides=xpadneo-dkms` and `conflicts=xpadneo-dkms`, so the AUR version
  cannot overwrite it during an ordinary AUR upgrade.
- DKMS rebuilds its module after installed kernels/headers change. Headers
  must be installed for every bootable kernel used with the controller.
- The patch and module option are owned by the Nodalix package and carried
  forward when Nodalix publishes an updated driver package.
- **New upstream driver versions are not applied blindly:** the pinned commit,
  patch, kernel build and controller behavior must be retested first.
- This is an out-of-tree DKMS driver, not a patch to the Linux kernel itself.
  Secure Boot systems that enforce signed modules may require module signing.

When migrating from the AUR `xpadneo-dkms` package, Pacman will ask to
replace the conflicting package. The installed Bluetooth pairing is
unaffected. An existing manual
`/etc/modprobe.d/99-nodalix-xpadneo-real-id.conf` from an earlier setup
shadows the packaged vendor file with the same name; it can be removed after
confirming the packaged `/usr/lib/modprobe.d/` configuration exists.

## Verify after installation

Turn on the Xbox controller, then run:

```sh
pacman -Q nodalix-xpadneo-dkms
dkms status | grep hid-xpadneo
modinfo -p hid_xpadneo | grep disable_pid_spoofing
cat /sys/module/hid_xpadneo/parameters/disable_pid_spoofing
grep -B1 'N: Name="Xbox Wireless Controller"' /proc/bus/input/devices
```

For a Microsoft BLE controller with PID `0b20`, expect a loaded parameter
of `Y` and `Bus=0005 Vendor=045e Product=0b20`. Check its buttons, sticks,
triggers, rumble, Bluetooth reconnection, and Steam Input in actual games.
The reported name in Steam may still depend on Steam's device database.

## Rollback and local override

To restore upstream's identity spoofing **without replacing the driver**,
create this file (an admin override takes precedence over the packaged
vendor config):

```sh
sudo install -d /etc/modprobe.d
printf 'options hid_xpadneo disable_pid_spoofing=0\n' |
  sudo tee /etc/modprobe.d/99-nodalix-xpadneo-real-id.conf
```

Reboot after switching. To re-enable native identity, change `0` to `1`
or delete the admin override and reboot. Do not unload a live gamepad module
while using it.

For a complete rollback, remove `nodalix-xpadneo-dkms` and reinstall the
original `xpadneo-dkms` package from the AUR, after reviewing its PKGBUILD.
A future Nodalix release may reinstall the native driver if it is a required
release component.

## Limitations

- The original ID behavior is currently validated only for Microsoft BLE
  `045e:0b20`; it is **not** a promise of full compatibility for every Xbox
  variant or game.
- Auto-reconnection is principally managed by BlueZ and controller firmware,
  and is separate from the PID fix.
- Linux kernel upgrades require matching headers for successful DKMS
  rebuilds. Inspect Pacman/DKMS logs before rebooting if the build fails.
- Review the upstream proposal:
  https://github.com/atar-axis/xpadneo/pull/603
