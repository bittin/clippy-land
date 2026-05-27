set dotenv-load := false

name    := 'cosmic-applet-clippy-land'
appid   := 'io.github.k33wee.clippy-land'
prefix  := '/usr'

bin_dir      := prefix + '/bin'
app_dir      := prefix + '/share/applications'
icon_dir     := prefix + '/share/icons/hicolor/scalable/apps'
metainfo_dir := prefix + '/share/metainfo'
license_dir  := prefix + '/share/licenses/' + appid

# default recipe
_default:
    @just --list

# Build release binary
build *args:
    cargo build --release {{args}}

# Alias for Flatpak compatibility
build-release *args:
    just build {{args}}

# Install (supports `just prefix=/app install` for Flatpak builds)
install:
    install -Dm755 target/release/{{name}}          {{bin_dir}}/{{name}}
    install -Dm755 resources/{{name}}.sh            {{bin_dir}}/{{name}}.sh
    install -Dm644 resources/{{appid}}.desktop      {{app_dir}}/{{appid}}.desktop
    install -Dm644 resources/{{appid}}.metainfo.xml {{metainfo_dir}}/{{appid}}.metainfo.xml
    install -Dm644 resources/icon.svg               {{icon_dir}}/{{appid}}.svg
    install -Dm644 resources/icon.svg               {{icon_dir}}/{{appid}}-symbolic.svg
    install -Dm644 LICENSE                          {{license_dir}}/LICENSE
    update-desktop-database {{app_dir}} || true
    gtk-update-icon-cache -f {{prefix}}/share/icons/hicolor || true

# Uninstall
uninstall:
    rm -f {{bin_dir}}/{{name}}
    rm -f {{bin_dir}}/{{name}}.sh
    rm -f {{app_dir}}/{{appid}}.desktop
    rm -f {{metainfo_dir}}/{{appid}}.metainfo.xml
    rm -f {{icon_dir}}/{{appid}}.svg
    rm -f {{icon_dir}}/{{appid}}-symbolic.svg
    rm -f {{license_dir}}/LICENSE
    update-desktop-database {{app_dir}} || true
    gtk-update-icon-cache -f {{prefix}}/share/icons/hicolor || true

# Clean build artifacts
clean:
    cargo clean

# Run unit/integration tests
test:
    cargo test

# Run Wayland clipboard round-trip unit tests (ignored by default)
test-wayland:
    CLIPPY_LAND_RUN_WAYLAND_TESTS=1 cargo test -- --ignored

# Run UI-level Wayland E2E clipboard tests via --toggle
e2e:
    ./tests/e2e/run.sh
