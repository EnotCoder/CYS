# Packaging

CYS ships as three artifacts, all built locally from the same source tree:

| Artifact | Built by | Target |
|---|---|---|
| `cys-<version>-linux-<arch>.tar.gz` | `packaging/build-linux.sh` | any Linux distribution |
| `cys_<version>_<arch>.deb` | `cargo deb` via the same script | Debian / Ubuntu |
| `CYS-<version>-setup.exe` | `packaging/build-windows.ps1` + Inno Setup 6 | Windows 10/11 |

The release itself is published manually: attach these files to a GitHub
release (or any file host) and give users the direct download links.

## Runtime paths

The binary no longer assumes that the current directory is the project root.
`src/core/asset.rs` resolves files as follows:

- **read-only assets** (`assets/...`): `CYS_ASSETS` (if set), then `assets/`
  next to the executable, then `/usr/share/cys/assets`, then `./assets`;
- **writable content** (`data/`, `mods/`, `texture_pack/`, `shaders/`):
  `CYS_HOME` if set, otherwise `$XDG_DATA_HOME/cys` (fallback
  `~/.local/share/cys`) on Linux and `%APPDATA%\CYS` on Windows.
  When launched from a source checkout (`Cargo.toml` and `assets/` in the
  current directory), the project root is used instead, so `cargo run` and
  the test suite behave exactly as before.

Bundled mods, texture packs and shaders stay readable because `list_dir()`
merges the user directory with the shipped `assets/` directory. Enabling a
mod writes an override `mod.json` into the user directory, never into the
system-wide installation.

## Linux tarball and `install.sh`

```sh
./packaging/build-linux.sh
tar -tzf dist/cys-0.1.3-linux-x86_64.tar.gz
```

The tarball contains the binary, `assets/`, `install.sh`, a `.desktop` file,
the icon, the license and the README. On the user's machine:

```sh
tar -xzf cys-0.1.3-linux-x86_64.tar.gz
cd cys-0.1.3-linux-x86_64
./install.sh                 # installs into ~/.local, no root needed
./install.sh --system        # or: sudo ./install.sh --system (prefix /usr/local)
./install.sh --prefix=/opt/cys
./install.sh --uninstall
```

`install.sh` puts the binary in `<prefix>/share/cys/cys` and links it into
`<prefix>/bin/cys`, installs the desktop entry and the 512×512 icon, and
refreshes the desktop database when the tools are available. Saves are kept
outside the prefix, so uninstalling does not delete worlds.

## Debian package

`Cargo.toml` contains `[package.metadata.deb]` with the binary, game assets,
desktop entry, icon and GPL license. Install the tool once:

```sh
cargo install cargo-deb
```

Then build from the project root:

```sh
./packaging/build-linux.sh
sudo apt install ./dist/cys_0.1.3_amd64.deb
cys
```

`cargo deb` runs the release build itself when invoked directly; the script
uses `--no-build` because it has just built the binary. The package depends
on `$auto` plus `libvulkan1` and `libasound2` (wgpu and rodio load those at
runtime). Remove with `sudo apt remove cys`.

`--output` places the `.deb` directly into `dist/`. To inspect the contents
before installing: `dpkg -c dist/cys_0.1.3_amd64.deb`.

## Windows installer

Build natively on Windows so wgpu keeps all of its default backends:

```powershell
.\packaging\build-windows.ps1
```

The script builds `target\release\cys.exe` and runs Inno Setup 6 with
`packaging\windows\cys.iss`, producing `dist\CYS-0.1.3-setup.exe`. If Inno
Setup is missing, the script prints the manual command instead:

```powershell
iscc packaging\windows\cys.iss /DMyAppVersion=0.1.3
```

The installer uses `PrivilegesRequired=lowest`, installs to
`%LOCALAPPDATA%\Programs\CYS`, adds a Start Menu entry, an optional desktop
shortcut, a license page and a normal uninstaller. `build/app.ico` is used
for both the installer and the game executable. Saves and settings go to
`%APPDATA%\CYS\data`, so uninstalling keeps them.

## Publishing a release

1. Run `packaging/build-linux.sh` on Linux and `packaging/build-windows.ps1`
   on Windows.
2. Optionally generate checksums: `sha256sum dist/* > dist/SHA256SUMS` (on
   Windows use `Get-FileHash`).
3. Create a GitHub release, set the tag to `v0.1.3`, and upload the tarball,
   the `.deb`, the setup `.exe` and the checksums.
4. Users can then download e.g.
   `https://github.com/<owner>/<repo>/releases/latest/download/cys-0.1.3-linux-x86_64.tar.gz`
   and run `tar -xzf ... && cd ... && ./install.sh`.

No CI workflow or apt repository is required: `apt install ./file.deb`
handles local packages, and GitHub Releases provides stable download URLs.
