# DroidForge Plus

DroidForge Plus is a desktop toolkit for Android Platform-Tools, built with Rust and egui.

## Features

### ADB
- Install APK files and run recovery sideload
- Push and pull files
- Create a local backup of shared internal storage (/sdcard/)
- Restore a backup with an explicit confirmation step
- Capture PNG screenshots and export Logcat
- Read Android build/device properties
- Reboot to system, recovery, or bootloader
- Show installed ADB and Fastboot versions

### Fastboot
- Flash and temporarily boot image files
- Erase partitions
- Unlock or relock the bootloader
- Reboot the device and query Fastboot variables
- Explicit confirmation before flashing, erasing, or changing bootloader state

### Reliability
- ADB detection ignores unauthorized and offline devices
- Fastboot detection requires a valid serial/state row
- Unit tests cover device-state parsing
- Cross-platform CI checks Windows and Linux

## Requirements
- Windows, Linux, or macOS
- Android SDK Platform-Tools (ADB and Fastboot) available in PATH
- A USB connection with USB debugging enabled, or a supported Fastboot connection

## Downloads: portable and installed editions

Every release provides **both portable and installable editions**:

| Platform | Portable edition | Installer |
|---|---|---|
| Windows x64 | ZIP archive | Setup EXE (Inno Setup) |
| Linux x64 | TAR.GZ archive | Debian package (.deb) |
| macOS x64 | TAR.GZ archive | Installer package (.pkg) |

**Portable:** extract the archive and run the executable. It can be kept in any folder and does not need installation.

**Installed:** run the platform installer. Windows creates a Start Menu shortcut and offers a desktop shortcut; Linux installs the app and command; macOS installs the app bundle into Applications.

### Updating

Download the same edition from the newer release. For portable installs, close the app and replace its files. For installed editions, run the newer installer/package over the existing installation. Do not uninstall the previous version first. Keep personal backups outside the application installation folder; updates do not intentionally remove them.

## Build from source

```bash
git clone https://github.com/mieson656-cell/Droid-ForgePlus.git
cd Droid-ForgePlus
cargo build --release
cargo run --release
```

Run checks locally:

```bash
cargo fmt --all -- --check
cargo test --all-targets
cargo check --all-targets
cargo build --release
```

## Safety notes
Flashing and erasing partitions can make a device unbootable. Verify the device model, partition layout, and image compatibility before proceeding. Unlocking the bootloader commonly wipes user data. Relocking a bootloader while using an incompatible or modified system can brick the device.

The shared-storage backup covers /sdcard/ only. It is not a complete device image and does not back up private app data, system partitions, or the bootloader. Restore can overwrite files with matching names.

## License

License terms are being verified against the upstream project before redistribution.
