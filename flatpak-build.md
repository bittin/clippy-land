# Prerequisites

- Install Flatpak and flatpak-builder (version >= 1.4.2)
- Install Flatpak SDKs: `org.freedesktop.Sdk` and `org.freedesktop.Platform`

## Quick start (fresh clone)

```bash
# 1. Validate the MetaInfo file
flatpak install -y --user flathub org.flatpak.Builder
flatpak run --command=flatpak-builder-lint org.flatpak.Builder \
  appstream resources/io.github.k33wee.clippy-land.metainfo.xml

# 2. Validate the desktop file
desktop-file-validate resources/io.github.k33wee.clippy-land.desktop

# 3. Install prerequisites
sudo apt-get install flatpak-builder
flatpak remote-add --if-not-exists --user flathub https://flathub.org/repo/flathub.flatpakrepo
flatpak remote-add --if-not-exists --user cosmic https://apt.pop-os.org/cosmic/cosmic.flatpakrepo
flatpak install -y --user flathub org.freedesktop.Sdk//25.08 org.freedesktop.Platform//25.08
flatpak install -y --user flathub org.freedesktop.Sdk.Extension.rust-stable//25.08
flatpak install -y --user cosmic com.system76.Cosmic.BaseApp//stable

# 4. Build locally (from the clippy-land project root):
flatpak-builder \
  --user \
  --install \
  --force-clean \
  --install-deps-from=flathub \
  build-dir \
  io.github.k33wee.clippy-land.json

# 5. Run it
flatpak run --user io.github.k33wee.clippy-land

# 6. Run the manifest linter
flatpak run --command=flatpak-builder-lint org.flatpak.Builder \
  manifest io.github.k33wee.clippy-land.json
```
