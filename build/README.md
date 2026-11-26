# Build and packaging tools

All tooling that prepares, builds or packages CYS lives under `build/`.
`Cargo.toml` stays at the repository root because Cargo requires it there.

| Path | Role |
|---|---|
| `cargo/main.rs` | Cargo build-script entry (`package.build` in `Cargo.toml`) |
| `cargo/windows.rs` | embeds `icons/app.ico` into the Windows `.exe` via `winres` |
| `icons/app.ico` | Windows executable and installer icon |
| `icons/icon.png` | Linux desktop icon (tarball, `.deb`) |
| `icons/android/` | Android launcher icons (`resources` in `Cargo.toml`) |
| `linux/build.sh` | builds the release binary, tarball and `.deb` |
| `linux/install.sh` | end-user installer, staged at the tarball root |
| `linux/cys.desktop` | desktop entry for the tarball and `.deb` |
| `windows/build.ps1` | builds the Windows binary and runs Inno Setup |
| `windows/cys.iss` | Inno Setup script |

Run the scripts directly from the new location:

```sh
./build/linux/build.sh
```

```powershell
.\build\windows\build.ps1
```

For compatibility, the old commands still work: `./install.sh`,
`./packaging/build-linux.sh` and `./packaging/build-windows.ps1` are thin
wrappers around the scripts above. Release tarballs still contain `install.sh`
at their root, so end users are unaffected.

Player mods, shaders and texture packs are shipped inside `assets/` and read
from `assets/mods/`, `assets/shaders/` and `assets/texture_pack/` on both
desktop and Android. They are not part of this build-tools folder.
