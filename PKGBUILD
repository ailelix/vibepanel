# Based on https://aur.archlinux.org/packages/vibepanel-git
# Maintainer: Felix Chen <felixchen531@gmail.com>
pkgname=vibepanel-alx
_pkgname=vibepanel
pkgver=0.15.0.r333.gc8ea9f5
pkgrel=1
pkgdesc="A personal VibePanel fork with ASUS and systemd VPN controls"
arch=('x86_64' 'aarch64')
url="https://github.com/ailelix/vibepanel"
license=('MIT')
depends=('gtk4' 'gtk4-layer-shell' 'libpulse' 'upower' 'networkmanager' 'bluez' 'systemd-libs')
makedepends=('git' 'cargo' 'rust' 'pkg-config')
optdepends=('power-profiles-daemon: power profile switching in battery popover'
            'modemmanager: cellular/mobile network support'
            'cava: audio visualizer in the media widget'
            'iwd: alternative to NetworkManager for Wi-Fi'
            'asusctl: ASUS fan profile control'
            'supergfxctl: ASUS graphics mode control'
            'wireguard-tools: wg-quick service managed by the VPN control widget'
            'sing-box: sing-box service managed by the VPN control widget'
            'sudo: default non-interactive privilege escalation for VPN service control')
options=(!lto !debug)
provides=("${_pkgname}")
conflicts=("${_pkgname}" "${_pkgname}-bin" "${_pkgname}-git")

_repo_url="${VIBEPANEL_REPO_URL:-https://github.com/ailelix/vibepanel.git#branch=main}"
source=("${_pkgname}::git+${_repo_url}")
sha256sums=('SKIP')

pkgver() {
  cd "${_pkgname}"

  local version
  version=$(sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -n 1)
  printf '%s.r%s.g%s\n' \
    "${version}" \
    "$(git rev-list --count HEAD)" \
    "$(git rev-parse --short=7 HEAD)"
}

prepare() {
  cd "${_pkgname}"
  export CARGO_HOME="${srcdir}/cargo"
  cargo fetch --locked --target "$(rustc -vV | sed -n 's/host: //p')"
}

build() {
  cd "${_pkgname}"
  export CARGO_HOME="${srcdir}/cargo"
  cargo build --release --frozen -p "${_pkgname}"
}

package() {
  cd "${_pkgname}"
  install -Dm755 "target/release/${_pkgname}" "${pkgdir}/usr/bin/${_pkgname}"
  install -Dm644 LICENSE "${pkgdir}/usr/share/licenses/${pkgname}/LICENSE"
}
