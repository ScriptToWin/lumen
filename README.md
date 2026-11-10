# Lumen

A lightweight, clean, Apple-inspired MP3 & WAV music player built in Rust with `eframe`/`egui` and `rodio`.

## Features
- **Clean 3-Column Desktop Layout:** Navigation & Playlists on the left, track list in the center, and Now Playing deck on the right.
- **Automatic Music Library:** Scans `~/Music` (including subfolders as playlists) and falls back to `./music`.
- **Embedded Cover Art:** Extracts album artwork directly from MP3 ID3 tags with clean fallback icons.
- **Multi-Language UI:** Switch seamlessly between English and German in the Settings tab.
- **Full Unicode & Japanese Support:** Renders Hiragana, Katakana, Kanji, and crisp system typography.

---

## System Dependencies

Before compiling from source, install the Rust toolchain (`rustup` / `cargo`) and the required native system libraries for audio (ALSA) and windowing (Wayland / X11 / OpenGL).

### Fedora / RHEL
```bash
sudo dnf install -y \
  alsa-lib-devel \
  pkgconf-pkg-config \
  libxkbcommon-devel \
  wayland-devel \
  libX11-devel \
  libXcursor-devel \
  libXrandr-devel \
  libXi-devel \
  mesa-libGL-devel \
  google-noto-sans-cjk-ttc-fonts \
  rsms-inter-fonts
```

### Ubuntu / Debian
```bash
sudo apt update && sudo apt install -y \
  libasound2-dev \
  pkg-config \
  libxkbcommon-dev \
  libwayland-dev \
  libx11-dev \
  libxcursor-dev \
  libxrandr-dev \
  libxi-dev \
  libgl1-mesa-dev \
  fonts-noto-cjk \
  fonts-inter
```

### Arch Linux
```bash
sudo pacman -S --needed \
  alsa-lib \
  pkgconf \
  libxkbcommon \
  wayland \
  libx11 \
  libxcursor \
  libxrandr \
  libxi \
  mesa \
  noto-fonts-cjk \
  inter-font
```

---

## Building as a Native Binary

1. Clone or enter the repository directory:
   ```bash
   cd lumen
   ```
2. Compile the optimized release binary:
   ```bash
   cargo build --release
   ```
3. Run the compiled binary directly:
   ```bash
   ./target/release/lumen
   ```
*(Optional)* Install it system-wide into `/usr/local/bin`:
```bash
sudo cp target/release/lumen /usr/local/bin/lumen
```

---

## Building as an AppImage (x86_64)

1. Build the release binary first:
   ```bash
   cargo build --release
   ```
2. Create the `AppDir` structure:
   ```bash
   mkdir -p Lumen.AppDir/usr/bin
   cp target/release/lumen Lumen.AppDir/usr/bin/lumen
   ```
3. Create the `.desktop` entry inside `Lumen.AppDir/lumen.desktop`:
   ```ini
   [Desktop Entry]
   Type=Application
   Name=Lumen
   Comment=Lightweight Apple-inspired Music Player
   Exec=lumen
   Icon=lumen
   Categories=AudioVideo;Audio;Player;
   Terminal=false
   ```
4. Add a `256x256` PNG icon named `lumen.png` inside `Lumen.AppDir/` and create an executable `AppRun` script:
   ```bash
   cat << 'EOF' > Lumen.AppDir/AppRun
   #!/bin/sh
   HERE="\$(dirname "(readlink -f "{0}")")"
   exec "HERE/usr/bin/lumen" "@"
   EOF
   chmod +x Lumen.AppDir/AppRun
   ```
5. Download `appimagetool` and package the AppImage:
   ```bash
   wget -O appimagetool https://github.com
   chmod +x appimagetool
   ARCH=x86_64 ./appimagetool Lumen.AppDir Lumen-x86_64.AppImage
   ```
6. Run your portable AppImage:
   ```bash
   chmod +x Lumen-x86_64.AppImage
   ./Lumen-x86_64.AppImage
   ```

---

## License & Rights

Copyright © 2026 **Ceyro**. All rights reserved.
