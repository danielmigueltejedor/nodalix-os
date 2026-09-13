pkgname=nodalix-shell
pkgver=0.1.1
pkgrel=4
pkgdesc="Nodalix desktop shell"
arch=("x86_64")
url="https://github.com/danielmigueltejedor/nodalix-os"
license=("GPL-3.0-or-later")
depends=("quickshell")
options=("!debug")
source=("nodalix-shell-$pkgver.tar.zst" "nodalix-shell" "VERSION")
sha256sums=("SKIP" "SKIP" "SKIP")

package() {
    install -d "$pkgdir/etc/xdg/quickshell/nodalix"
    cp -a "$srcdir/shell/." "$pkgdir/etc/xdg/quickshell/nodalix/"
    install -Dm755 "$srcdir/nodalix-shell" "$pkgdir/usr/bin/nodalix-shell"
    install -Dm644 "$srcdir/VERSION" "$pkgdir/etc/xdg/quickshell/nodalix/VERSION"
}
