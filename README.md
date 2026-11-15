# DroidForge Plus

A lightweight Rust desktop GUI for Android Debug Bridge (ADB) and Fastboot.

## Features

### ADB
- Install APK files
- Capture screenshots
- Push and pull files
- ADB sideload
- Export Logcat
- Read device properties
- Reboot to system, recovery, or bootloader

### Fastboot
- Flash partition images
- Temporarily boot an image
- Unlock or relock the bootloader
- Erase partitions
- Reboot device
- Read Fastboot variables

## Requirements

- Rust stable toolchain
- Android SDK Platform-Tools (ADB and Fastboot)
- Windows or Linux

Make sure adb and fastboot are available in your PATH.

## Build

\`\`\`bash
git clone https://github.com/mieson656-cell/Droid-ForgePlus.git
cd Droid-ForgePlus
cargo build --release
cargo run --release
\`\`\`

## Safety

Flashing, erasing partitions, and changing bootloader state can permanently damage a device or erase user data. Verify the device model, partition layout, and image compatibility before proceeding. Bootloader unlock usually performs a factory reset. Never relock the bootloader while running an incompatible or modified system.

## License

License terms are being verified against the upstream project before redistribution.
