# clippy-land

![GitHub License](https://img.shields.io/github/license/k33wee/clippy-land)
![GitHub Repo stars](https://img.shields.io/github/stars/k33wee/clippy-land)

COSMIC panel applet for keeping a history of recently copied text and images.

This applet polls the Wayland clipboard and updates the history when
the contents change.

![applet example](./resources/example.png)

## Main features

- Keep a history of the latest **30** clipboard entries (text + images)
- Re-copy an entry with a single click
- Remove individual entries from the history
- Clear all entries from history with one click
- Pin important entries to the top of the history (5 pinned entries max)
- Move between entries with arrow keys ( **up/down** or **k/j** to navigate between entries, **left/right** or **h/l** to move to pin or delete buttons. You need to interact with the applet at least once to enable keyboard navigation, or open it with a custom shortcut)
- Adds keyboard shortcuts for opening the history (see [Usage](#usage) below)

## Table of Contents

- [Install from cosmic store](#cosmic-store)
- [Usage](#usage)
- [Install with Flatpak](#install-with-flatpak)
- [Install for Debian/Ubuntu](#install-for-debianubuntu)
- [Install for Fedora](#install-for-fedora)
- [Build/Install with just](#buildinstall-with-just)
- [Install with custom paths](#install-with-custom-paths)
- [Notes](#notes)
- [Translations](#translations)

## Cosmic Store

The applet is officially pubished on [Cosmic Store](https://github.com/pop-os/cosmic-store). In COSMIC Store it should be under the "COSMIC Applets" category.

If it does not show up in your app store, you'll need to add `cosmic-flatpak` as a source:

```sh
flatpak remote-add --if-not-exists --user cosmic https://apt.pop-os.org/cosmic/cosmic.flatpakrepo
```

Just install it from the store and you'll have the flatpak sandbox installed!

## Usage

Open **COSMIC Settings → Desktop → Panel → Applets** and add "Clippy Land" to your panel.
You might need to log out and back in to see the applet in the list of available applets.

## Keyboard Shortcut

You can open the clipboard history with a keyboard shortcut via the `--toggle` flag.

Go to **COSMIC Settings → Keyboard → Custom Shortcuts**, add a new shortcut with:

- **Name:** Toggle ClippyLand
- **Command:** `cosmic-applet-clippy-land --toggle` (or `flatpak run io.github.k33wee.clippy-land --toggle` if installed via Flatpak)
- **Shortcut:** your preferred key combo (e.g. `Super+V`)

> **Note:** Due to a current limitation in the COSMIC panel, opening a popup without a pointer event (i.e. without actually clicking the applet icon) is not natively supported. As a workaround, `--toggle` opens the history as a full-width layer surface anchored to the top of the screen rather than as the usual dropdown under the icon. This is a known limitation and may be improved in the future once COSMIC panel exposes a proper API for this.

## Install with Flatpak

1. Add the required remotes (if not already added):

```bash
flatpak remote-add --if-not-exists flathub https://flathub.org/repo/flathub.flatpakrepo
```

2. Download the latest `.flatpak` from the [releases page](https://github.com/k33wee/clippy-land/releases/).
3. In a terminal, navigate to the directory where you downloaded the `.flatpak` file and run:

```bash
flatpak install --user ./clippy-land_<version>.flatpak
```

This will install the applet for your user. If you encounter missing runtime errors, ensure the remotes above are added and try again.

## Install for Debian/Ubuntu

Download the latest .deb from the [releases page](https://github.com/k33wee/clippy-land/releases/):
In a terminal, navigate to the directory where you downloaded the .deb file and run:

```bash
sudo apt install ./cosmic-applet-clippy-land_<version>_amd64.deb
```

## Install for Fedora

Thanks to [lorduskordus](https://github.com/lorduskordus) there is now an RPM package on COPR.

- [copr.fedorainfracloud.org/coprs/kordus/cosmic-applets](https://copr.fedorainfracloud.org/coprs/kordus/cosmic-applets)

Traditional Fedora

```sh
sudo dnf copr enable kordus/cosmic-applets
sudo dnf install cosmic-applet-clippy-land
```

Fedora Atomic

```sh
sudo wget \
    https://copr.fedorainfracloud.org/coprs/kordus/cosmic-applets/repo/fedora/kordus-cosmic-applets.repo \
    -O /etc/yum.repos.d/_copr:copr.fedorainfracloud.org:kordus:cosmic-applets.repo
rpm-ostree install cosmic-applet-clippy-land
```

## Build/Install with just

### Dependencies

- Wayland clipboard support (via `wl-clipboard-rs`)
- Build dependencies for libcosmic:

```bash
sudo apt install cargo cmake just libexpat1-dev libfontconfig-dev libfreetype-dev libxkbcommon-dev pkgconf
```

```bash
# build
sudo just build

# install for current user
sudo just install
```

## Install with custom paths

Pass a `prefix` variable to install everything under a custom root:

```bash
# install under ~/.local  (default is /usr)
sudo just prefix=~/.local install

# uninstall
sudo just prefix=~/.local uninstall
```

All paths are derived from `prefix`:

| Path                                         | Default                  |
| -------------------------------------------- | ------------------------ |
| `<prefix>/bin`                               | binary + launcher script |
| `<prefix>/share/applications`                | `.desktop` file          |
| `<prefix>/share/icons/hicolor/scalable/apps` | app icon                 |
| `<prefix>/share/metainfo`                    | MetaInfo file            |
| `<prefix>/share/licenses/<appid>`            | license                  |

## Notes

- App ID is currently `io.github.k33wee.clippy-land`.

## Translations

Clippy Land is available in the following languages, thanks to:

- **Italian** ([k33wee](https://github.com/k33wee))
- **English** ([k33wee](https://github.com/k33wee))
- **Portuguese** ([GuilhermeTerriaga](https://github.com/GuilhermeTerriaga))
- **Czech** ([lorduskordus](https://github.com/lorduskordus))
- **Ukrainian** ([Dymkom](https://github.com/Dymkom))
- **Swedish** ([bittin](https://github.com/bittin))
