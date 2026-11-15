use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

/// Prefer the copy bundled beside the application; fall back to PATH for developer setups.
fn platform_tool_path(cmd: &str) -> PathBuf {
    let executable = std::env::current_exe().ok();
    if let Some(parent) = executable.as_ref().and_then(|p| p.parent()) {
        let bundled = parent.join("platform-tools").join(if cfg!(windows) {
            format!("{cmd}.exe")
        } else {
            cmd.to_string()
        });
        if bundled.is_file() {
            return bundled;
        }
        // macOS .app layout: Contents/MacOS/<app>, resources live in Contents/Resources.
        let mac_bundle = parent.join("../Resources/platform-tools").join(cmd);
        if mac_bundle.is_file() {
            return mac_bundle;
        }
    }
    PathBuf::from(cmd)
}

/// Run an external Android platform-tools command without invoking a shell.
pub fn run_command(cmd: &str, args: &[&str]) -> Result<String, String> {
    let tool = platform_tool_path(cmd);
    let output = Command::new(&tool)
        .args(args)
        .stdin(Stdio::null())
        .output()
        .map_err(|e| format!("Не удалось запустить {cmd}: {e}. Проверьте установку Android Platform-Tools и PATH."))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = match (stdout.trim(), stderr.trim()) {
        ("", "") => String::from("Команда выполнена без вывода."),
        ("", err) => err.to_owned(),
        (out, "") => out.to_owned(),
        (out, err) => format!("{out}\n{err}"),
    };

    if output.status.success() {
        Ok(combined)
    } else {
        Err(format!("Команда завершилась с кодом {:?}:\n{combined}", output.status.code()))
    }
}


/// Return true only for an authorized ADB device row.
fn has_ready_adb_device(output: &str) -> bool {
    output.lines().any(|line| {
        let mut fields = line.split_whitespace();
        matches!((fields.next(), fields.next()), (Some(serial), Some("device")) if serial != "List")
    })
}

/// Return true only for a serial followed by Fastboot's device state.
fn has_fastboot_device(output: &str) -> bool {
    output.lines().any(|line| {
        let mut fields = line.split_whitespace();
        matches!((fields.next(), fields.next()), (Some(serial), Some("fastboot")) if !serial.is_empty())
    })
}

/// Parse adb devices and only accept devices in the ready device state.
pub fn check_adb() -> (bool, String) {
    match run_command("adb", &["devices", "-l"]) {
        Ok(output) => {
            let found = has_ready_adb_device(&output);
            (found, output)
        }
        Err(e) => (false, e),
    }
}

/// Parse fastboot devices; a non-empty error message is not a connected device.
pub fn check_fastboot() -> (bool, String) {
    match run_command("fastboot", &["devices"]) {
        Ok(output) => {
            let found = has_fastboot_device(&output);
            (found, output)
        }
        Err(e) => (false, e),
    }
}

pub fn install_apk(file_path: &str) -> Result<String, String> {
    run_command("adb", &["install", "-r", file_path])
}
pub fn sideload(file_path: &str) -> Result<String, String> {
    run_command("adb", &["sideload", file_path])
}
pub fn push_file(file_path: &str, dest: &str) -> Result<String, String> {
    run_command("adb", &["push", file_path, dest])
}
pub fn pull_file(remote_path: &str, local_path: &str) -> Result<String, String> {
    run_command("adb", &["pull", remote_path, local_path])
}
pub fn adb_reboot(mode: &str) -> Result<String, String> {
    if mode.is_empty() { run_command("adb", &["reboot"]) }
    else { run_command("adb", &["reboot", mode]) }
}

/// Capture binary PNG output directly; shell redirection does not work with Command.
pub fn screenshot(output_path: &str) -> Result<String, String> {
    let output = Command::new("adb")
        .args(["exec-out", "screencap", "-p"])
        .stdin(Stdio::null())
        .output()
        .map_err(|e| format!("Не удалось запустить adb: {e}"))?;

    if !output.status.success() {
        return Err(format!("Ошибка создания скриншота: {}", String::from_utf8_lossy(&output.stderr)));
    }
    if output.stdout.is_empty() {
        return Err("ADB вернул пустой файл скриншота.".to_string());
    }
    fs::write(output_path, &output.stdout)
        .map_err(|e| format!("Не удалось сохранить скриншот: {e}"))?;
    Ok(format!("Скриншот сохранён: {} ({} байт)", output_path, output.stdout.len()))
}

pub fn logcat(output_path: &str) -> Result<String, String> {
    let logs = run_command("adb", &["logcat", "-d"])?;
    fs::write(output_path, &logs)
        .map_err(|e| format!("Ошибка записи файла: {e}"))?;
    Ok(format!("Логи сохранены: {} ({} байт)", output_path, logs.len()))
}

pub fn get_device_info() -> Result<String, String> {
    let fields = [
        ("Модель", "ro.product.model"),
        ("Производитель", "ro.product.manufacturer"),
        ("Устройство", "ro.product.device"),
        ("Версия Android", "ro.build.version.release"),
        ("Уровень API", "ro.build.version.sdk"),
        ("Архитектура", "ro.product.cpu.abi"),
        ("Отпечаток сборки", "ro.build.fingerprint"),
    ];
    let mut info = String::new();
    for (label, property) in fields {
        let value = run_command("adb", &["shell", "getprop", property])
            .unwrap_or_else(|_| "Недоступно".to_string());
        info.push_str(&format!("{label}: {}\n", value.trim()));
    }
    Ok(info)
}


/// Copy shared internal storage to a local directory using ADB's recursive pull.
pub fn backup_shared_storage(output_dir: &str) -> Result<String, String> {
    if output_dir.trim().is_empty() {
        return Err("Не выбрана папка для резервной копии.".into());
    }
    fs::create_dir_all(output_dir)
        .map_err(|e| format!("Не удалось создать папку резервной копии: {e}"))?;
    run_command("adb", &["pull", "/sdcard/", output_dir])
}

/// Restore a previously exported directory into shared internal storage.
/// Existing files with matching names may be overwritten by ADB.
pub fn restore_shared_storage(source_dir: &str) -> Result<String, String> {
    let source = Path::new(source_dir);
    if !source.is_dir() {
        return Err("Папка резервной копии не найдена.".into());
    }
    let source_with_contents = format!("{}/.", source_dir.trim_end_matches('/').trim_end_matches('\\'));
    run_command("adb", &["push", &source_with_contents, "/sdcard/"])
}

/// Report installed Android Platform-Tools versions.
pub fn get_platform_tools_versions() -> String {
    let adb = run_command("adb", &["version"])
        .unwrap_or_else(|e| format!("ADB: {e}"));
    let fastboot = run_command("fastboot", &["--version"])
        .unwrap_or_else(|e| format!("Fastboot: {e}"));
    format!("Версия ADB:\n{adb}\n\nВерсия Fastboot:\n{fastboot}")
}

pub fn flash_partition(partition: &str, image_path: &str) -> Result<String, String> {
    if partition.trim().is_empty() { return Err("Не выбран раздел для прошивки.".into()); }
    if !Path::new(image_path).is_file() { return Err("Файл образа не найден.".into()); }
    run_command("fastboot", &["flash", partition, image_path])
}
pub fn boot_image(image_path: &str) -> Result<String, String> {
    if !Path::new(image_path).is_file() { return Err("Файл образа не найден.".into()); }
    run_command("fastboot", &["boot", image_path])
}
pub fn unlock_bootloader() -> Result<String, String> {
    run_command("fastboot", &["flashing", "unlock"])
}
pub fn lock_bootloader() -> Result<String, String> {
    run_command("fastboot", &["flashing", "lock"])
}
pub fn erase_partition(partition: &str) -> Result<String, String> {
    if partition.trim().is_empty() { return Err("Не выбран раздел для очистки.".into()); }
    run_command("fastboot", &["erase", partition])
}
pub fn fastboot_reboot(mode: &str) -> Result<String, String> {
    if mode.is_empty() { run_command("fastboot", &["reboot"]) }
    else { run_command("fastboot", &["reboot", mode]) }
}
pub fn get_fastboot_var(var: &str) -> Result<String, String> {
    if var.trim().is_empty() { return Err("Укажите имя переменной Fastboot.".into()); }
    run_command("fastboot", &["getvar", var])
}

#[cfg(test)]
mod tests {
    use super::{has_fastboot_device, has_ready_adb_device};

    #[test]
    fn adb_accepts_only_authorized_device_rows() {
        assert!(has_ready_adb_device("List of devices attached\nR58M123 device product:a56\n"));
        assert!(!has_ready_adb_device("List of devices attached\nR58M123 unauthorized\n"));
        assert!(!has_ready_adb_device("List of devices attached\nR58M123 offline\n"));
        assert!(!has_ready_adb_device("List of devices attached\n"));
    }

    #[test]
    fn fastboot_requires_a_serial_and_state_field() {
        assert!(has_fastboot_device("ABC123 fastboot\n"));
        assert!(!has_fastboot_device("\n"));
        assert!(!has_fastboot_device("...\n"));
    }
}
