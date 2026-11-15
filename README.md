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

## Download and update
Download the archive for your operating system from the [Releases](https://github.com/mieson656-cell/Droid-ForgePlus/releases) page. Extract it and run the application.

DroidForge Plus is distributed as a portable application: to update, close it, replace the old executable with the one from the newer release, and launch it again. No uninstall step is required. Keep exported backups in a separate folder; replacing the application does not touch them.

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
