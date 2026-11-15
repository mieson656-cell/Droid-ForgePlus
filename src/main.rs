mod android;

use egui;
use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

#[derive(PartialEq)]
enum Tab {
    Adb,
    Fastboot,
}

#[derive(Clone)]
enum PendingAction {
    Flash { partition: String, image: String },
    Erase { partition: String },
    UnlockBootloader,
    LockBootloader,
    RestoreBackup { directory: String },
}

struct DroidForgeApp {
    current_tab: Tab,
    adb_connected: bool,
    fastboot_connected: bool,
    log_text: String,

    // ADB состояние
    selected_apk: Option<String>,
    selected_sideload: Option<String>,
    selected_push_file: Option<String>,
    pull_remote_path: String,

    // Fastboot состояние
    selected_partition: String,
    selected_image: Option<String>,
    partitions: Vec<String>,
    fastboot_var: String,
    pending_action: Option<PendingAction>,
}

impl Default for DroidForgeApp {
    fn default() -> Self {
        Self {
            current_tab: Tab::Adb,
            adb_connected: false,
            fastboot_connected: false,
            log_text: String::from("[ИНФО] Добро пожаловать в DroidForge Plus!\n[ИНФО] Нажмите 'Обновить статус' для проверки подключения\n"),

            selected_apk: None,
            selected_sideload: None,
            selected_push_file: None,
            pull_remote_path: String::from("/sdcard/"),

            selected_partition: "boot".to_string(),
            selected_image: None,
            partitions: vec![
                "boot".to_string(),
                "recovery".to_string(),
                "system".to_string(),
                "vendor".to_string(),
                "radio".to_string(),
            ],
            fastboot_var: "product".to_string(),
            pending_action: None,
        }
    }
}

impl DroidForgeApp {
    fn add_log(&mut self, msg: &str) {
        self.log_text.push_str(msg);
        self.log_text.push('\n');
    }

    fn check_connection_status(&mut self) {
        self.add_log("[ИНФО] Проверка подключения...");

        let (adb, adb_log) = android::check_adb();
        self.adb_connected = adb;
        if adb {
            self.add_log("[ADB] ✓ Устройство подключено в режиме ADB");
        } else {
            self.add_log("[ADB] Устройство не найдено или не подтверждена отладка по USB.");
            if adb_log.starts_with("Не удалось запустить") { self.add_log(&format!("[ADB] {adb_log}")); }
        }

        let (fb, fb_log) = android::check_fastboot();
        self.fastboot_connected = fb;
        if fb {
            self.add_log("[ЗАГРУЗЧИК] Устройство подключено в режиме Fastboot.");
        } else {
            self.add_log("[ЗАГРУЗЧИК] Устройство в режиме Fastboot не найдено.");
        }
        if !fb && fb_log.starts_with("Не удалось запустить") {
            self.add_log(&format!("[ЗАГРУЗЧИК] {fb_log}"));
        }
    }

    fn render_status_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let adb_color = if self.adb_connected {
                egui::Color32::from_rgb(34, 211, 238) // cyan
            } else {
                egui::Color32::from_rgb(100, 100, 100)
            };

            let fb_color = if self.fastboot_connected {
                egui::Color32::from_rgb(34, 211, 238)
            } else {
                egui::Color32::from_rgb(100, 100, 100)
            };

            ui.colored_label(adb_color, if self.adb_connected { "● ADB: Подключено" } else { "○ ADB: Отключено" });
            ui.separator();
            ui.colored_label(fb_color, if self.fastboot_connected { "● Загрузчик: Подключён" } else { "○ Загрузчик: Отключён" });

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("🔄 Обновить статус").clicked() {
                    self.check_connection_status();
                }
            });
        });
    }

    fn render_adb_tab(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            // Установка и файлы
            ui.group(|ui| {
                ui.heading("📦 Установка и файлы");
                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    if ui.button("Выбрать APK").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Файлы APK", &["apk"])
                            .pick_file()
                        {
                            self.selected_apk = Some(path.display().to_string());
                            self.add_log(&format!("[ИНФО] Выбран APK: {}", path.display()));
                        }
                    }

                    ui.add_enabled_ui(self.adb_connected && self.selected_apk.is_some(), |ui| {
                        if ui.button("⚡ Установить APK").clicked() {
                            if let Some(apk_path) = self.selected_apk.clone() {
                                self.add_log(&format!("[ADB] Установка APK: {}", apk_path));
                                match android::install_apk(&apk_path) {
                                    Ok(log) => self.add_log(&format!("[УСПЕХ] {}", log)),
                                    Err(e) => self.add_log(&format!("[ОШИБКА] {}", e)),
                                }
                            }
                        }
                    });

                    if let Some(apk) = &self.selected_apk {
                        ui.label(apk);
                    }
                });

                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    if ui.button("Выбрать ZIP").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("ZIP-архивы", &["zip"])
                            .pick_file()
                        {
                            self.selected_sideload = Some(path.display().to_string());
                            self.add_log(&format!("[ИНФО] Выбран ZIP: {}", path.display()));
                        }
                    }

                    ui.add_enabled_ui(self.adb_connected && self.selected_sideload.is_some(), |ui| {
                        if ui.button("⚡ Установить ZIP через Recovery").clicked() {
                            if let Some(zip_path) = self.selected_sideload.clone() {
                                self.add_log("⚠ Убедитесь, что устройство в Recovery!");
                                self.add_log(&format!("[ADB] Установка ZIP: {}", zip_path));
                                match android::sideload(&zip_path) {
                                    Ok(log) => self.add_log(&format!("[УСПЕХ] {}", log)),
                                    Err(e) => self.add_log(&format!("[ОШИБКА] {}", e)),
                                }
                            }
                        }
                    });

                    if let Some(zip) = &self.selected_sideload {
                        ui.label(zip);
                    }
                });

                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    if ui.button("Выбрать файл").clicked() {
                        if let Some(path) = rfd::FileDialog::new().pick_file() {
                            self.selected_push_file = Some(path.display().to_string());
                            self.add_log(&format!("[ИНФО] Выбран файл: {}", path.display()));
                        }
                    }

                    ui.add_enabled_ui(self.adb_connected && self.selected_push_file.is_some(), |ui| {
                        if ui.button("⬆ Передать файл на /sdcard/").clicked() {
                            if let Some(file_path) = &self.selected_push_file.clone() {
                                self.add_log(&format!("[ADB] Передача файла: {}", file_path));
                                match android::push_file(file_path, "/sdcard/") {
                                    Ok(log) => self.add_log(&format!("[УСПЕХ] {}", log)),
                                    Err(e) => self.add_log(&format!("[ОШИБКА] {}", e)),
                                }
                            }
                        }
                    });

                    if let Some(file) = &self.selected_push_file {
                        ui.label(file);
                    }
                });

                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.label("Путь на устройстве:");
                    ui.text_edit_singleline(&mut self.pull_remote_path);

                    ui.add_enabled_ui(self.adb_connected, |ui| {
                        if ui.button("⬇ Забрать файл").clicked() {
                            if let Some(path) = rfd::FileDialog::new().save_file() {
                                let local_path = path.display().to_string();
                                self.add_log(&format!("[ADB] Получение файла: {} -> {}", self.pull_remote_path, local_path));
                                match android::pull_file(&self.pull_remote_path, &local_path) {
                                    Ok(log) => self.add_log(&format!("[УСПЕХ] {}", log)),
                                    Err(e) => self.add_log(&format!("[ОШИБКА] {}", e)),
                                }
                            }
                        }
                    });
                });
            });

            ui.add_space(8.0);


            ui.add_space(8.0);
            ui.group(|ui| {
                ui.heading("Резервная копия");
                ui.label("Экспортирует содержимое /sdcard/ в выбранную папку на компьютере.");
                ui.horizontal(|ui| {
                    ui.add_enabled_ui(self.adb_connected, |ui| {
                        if ui.button("Создать копию").clicked() {
                            if let Some(path) = rfd::FileDialog::new().pick_folder() {
                                let directory = path.display().to_string();
                                self.add_log(&format!("[ADB] Резервное копирование /sdcard/ -> {directory}"));
                                match android::backup_shared_storage(&directory) {
                                    Ok(log) => self.add_log(&format!("[УСПЕХ] {log}")),
                                    Err(e) => self.add_log(&format!("[ОШИБКА] {e}")),
                                }
                            }
                        }
                        if ui.button("Восстановить копию").clicked() {
                            if let Some(path) = rfd::FileDialog::new().pick_folder() {
                                self.pending_action = Some(PendingAction::RestoreBackup { directory: path.display().to_string() });
                            }
                        }
                    });
                });
                ui.label("Восстановление копирует файлы в /sdcard/ и может перезаписать совпадающие файлы.");
            });

            // Ребут и режимы
            ui.group(|ui| {
                ui.heading("🔄 Перезагрузка и режимы");
                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    ui.add_enabled_ui(self.adb_connected, |ui| {
                        if ui.button("В систему").clicked() {
                            self.add_log("[ADB] Перезагрузка в систему...");
                            match android::adb_reboot("") {
                                Ok(log) => self.add_log(&format!("[УСПЕХ] {}", log)),
                                Err(e) => self.add_log(&format!("[ОШИБКА] {}", e)),
                            }
                        }

                        if ui.button("В Recovery").clicked() {
                            self.add_log("[ADB] Перезагрузка в Recovery...");
                            match android::adb_reboot("recovery") {
                                Ok(log) => self.add_log(&format!("[УСПЕХ] {}", log)),
                                Err(e) => self.add_log(&format!("[ОШИБКА] {}", e)),
                            }
                        }

                        if ui.button("В режим загрузчика").clicked() {
                            self.add_log("[ADB] Перезагрузка в режим загрузчика...");
                            match android::adb_reboot("bootloader") {
                                Ok(log) => self.add_log(&format!("[УСПЕХ] {}", log)),
                                Err(e) => self.add_log(&format!("[ОШИБКА] {}", e)),
                            }
                        }
                    });
                });
            });

            ui.add_space(8.0);

            // Информация и отладка
            ui.group(|ui| {
                ui.heading("🔍 Информация и отладка");
                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    ui.add_enabled_ui(self.adb_connected, |ui| {
                        if ui.button("📸 Скриншот").clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .set_file_name("снимок_экрана.png")
                                .add_filter("Изображения PNG", &["png"])
                                .save_file()
                            {
                                let output_path = path.display().to_string();
                                self.add_log("[ADB] Создание скриншота...");
                                match android::screenshot(&output_path) {
                                    Ok(log) => self.add_log(&format!("[УСПЕХ] Сохранено в {}\n{}", output_path, log)),
                                    Err(e) => self.add_log(&format!("[ОШИБКА] {}", e)),
                                }
                            }
                        }

                        if ui.button("📋 Журнал устройства").clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .set_file_name("журнал_устройства.txt")
                                .add_filter("Текстовые файлы", &["txt"])
                                .save_file()
                            {
                                let output_path = path.display().to_string();
                                self.add_log("[ADB] Сбор логов...");
                                match android::logcat(&output_path) {
                                    Ok(log) => self.add_log(&format!("[УСПЕХ] {}", log)),
                                    Err(e) => self.add_log(&format!("[ОШИБКА] {}", e)),
                                }
                            }
                        }

                        if ui.button("ℹ Инфо о телефоне").clicked() {
                            self.add_log("[ADB] Получение информации...");
                            match android::get_device_info() {
                                Ok(info) => self.add_log(&format!("[ИНФО]\n{}", info)),
                                Err(e) => self.add_log(&format!("[ОШИБКА] {}", e)),
                            }
                        }
                        if ui.button("Версии инструментов Android").clicked() {
                            self.add_log(&format!("[ИНСТРУМЕНТЫ]\n{}", android::get_platform_tools_versions()));
                        }
                    });
                });
            });
        });
    }

    fn render_fastboot_tab(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            // Прошивка
            ui.group(|ui| {
                ui.heading("⚡ Прошивка");
                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    ui.label("Раздел:");
                    egui::ComboBox::from_id_salt("partition_select")
                        .selected_text(&self.selected_partition)
                        .show_ui(ui, |ui| {
                            for partition in &self.partitions {
                                ui.selectable_value(
                                    &mut self.selected_partition,
                                    partition.clone(),
                                    partition
                                );
                            }
                        });
                });

                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    if ui.button("📁 Выбрать .img").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Файлы образов", &["img"])
                            .pick_file()
                        {
                            self.selected_image = Some(path.display().to_string());
                            self.add_log(&format!("[ИНФО] Выбран образ: {}", path.display()));
                        }
                    }

                    if let Some(img) = &self.selected_image {
                        ui.label(img);
                    }
                });

                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.add_enabled_ui(self.fastboot_connected && self.selected_image.is_some(), |ui| {
                        if ui.button("⚡ Прошить").clicked() {
                            if let Some(img_path) = self.selected_image.clone() {
                                let partition = self.selected_partition.clone();
                                self.pending_action = Some(PendingAction::Flash {
                                    partition,
                                    image: img_path,
                                });
                            }
                        }

                        if ui.button("🚀 Загрузиться с образа").clicked() {
                            if let Some(img_path) = self.selected_image.clone() {
                                self.add_log(&format!("[ЗАГРУЗЧИК] Запуск образа без прошивки: {}", img_path));
                                match android::boot_image(&img_path) {
                                    Ok(log) => self.add_log(&format!("[УСПЕХ] {}", log)),
                                    Err(e) => self.add_log(&format!("[ОШИБКА] {}", e)),
                                }
                            }
                        }
                    });
                });
            });

            ui.add_space(8.0);

            // Управление загрузчиком
            ui.group(|ui| {
                ui.heading("🔓 Управление загрузчиком");
                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    ui.add_enabled_ui(self.fastboot_connected, |ui| {
                        if ui.button("🔓 Разблокировать").clicked() {
                            self.pending_action = Some(PendingAction::UnlockBootloader);
                        }

                        if ui.button("🔒 Заблокировать").clicked() {
                            self.pending_action = Some(PendingAction::LockBootloader);
                        }
                    });
                });

                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.label("Стереть раздел:");
                    egui::ComboBox::from_id_salt("erase_partition")
                        .selected_text(&self.selected_partition)
                        .show_ui(ui, |ui| {
                            for partition in &self.partitions {
                                ui.selectable_value(
                                    &mut self.selected_partition,
                                    partition.clone(),
                                    partition
                                );
                            }
                        });

                    ui.add_enabled_ui(self.fastboot_connected, |ui| {
                        if ui.button("🗑 Стереть").clicked() {
                            self.pending_action = Some(PendingAction::Erase {
                                partition: self.selected_partition.clone(),
                            });
                        }
                    });
                });
            });

            ui.add_space(8.0);

            // Ребут
            ui.group(|ui| {
                ui.heading("🔄 Перезагрузка");
                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    ui.add_enabled_ui(self.fastboot_connected, |ui| {
                        if ui.button("Перезагрузить").clicked() {
                            self.add_log("[ЗАГРУЗЧИК] Перезагрузка...");
                            match android::fastboot_reboot("") {
                                Ok(log) => self.add_log(&format!("[УСПЕХ] {}", log)),
                                Err(e) => self.add_log(&format!("[ОШИБКА] {}", e)),
                            }
                        }

                        if ui.button("В Recovery").clicked() {
                            self.add_log("[ЗАГРУЗЧИК] Перезагрузка в Recovery...");
                            match android::fastboot_reboot("recovery") {
                                Ok(log) => self.add_log(&format!("[УСПЕХ] {}", log)),
                                Err(e) => self.add_log(&format!("[ОШИБКА] {}", e)),
                            }
                        }

                        if ui.button("В загрузчик").clicked() {
                            self.add_log("[ЗАГРУЗЧИК] Перезагрузка в режим загрузчика...");
                            match android::fastboot_reboot("bootloader") {
                                Ok(log) => self.add_log(&format!("[УСПЕХ] {}", log)),
                                Err(e) => self.add_log(&format!("[ОШИБКА] {}", e)),
                            }
                        }
                    });
                });
            });

            ui.add_space(8.0);

            // Запрос сведений Fastboot
            ui.group(|ui| {
                ui.heading("ℹ Информация");
                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    ui.label("Переменная:");
                    ui.text_edit_singleline(&mut self.fastboot_var);

                    ui.add_enabled_ui(self.fastboot_connected, |ui| {
                        if ui.button("📋 Получить переменную").clicked() {
                            self.add_log(&format!("[ЗАГРУЗЧИК] Запрос переменной {}...", self.fastboot_var));
                            match android::get_fastboot_var(&self.fastboot_var) {
                                Ok(log) => self.add_log(&format!("[ИНФО] {}", log)),
                                Err(e) => self.add_log(&format!("[ОШИБКА] {}", e)),
                            }
                        }
                    });
                });

                ui.add_space(4.0);
                ui.label("Примеры: product, version-bootloader, version-baseband, serialno");
            });
        });
    }
}

impl DroidForgeApp {
    fn update_ui(&mut self, ctx: &egui::Context) {
        ctx.request_repaint_after(std::time::Duration::from_millis(100));

        // Единая тёмная тема и удобные интервалы.
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = egui::Color32::from_rgb(15, 18, 25);
        visuals.window_fill = egui::Color32::from_rgb(22, 27, 38);
        visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(35, 43, 58);
        visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(48, 62, 82);
        visuals.widgets.active.bg_fill = egui::Color32::from_rgb(20, 115, 145);
        ctx.set_visuals(visuals);
        ctx.style_mut(|style| {
            style.spacing.item_spacing = egui::vec2(10.0, 8.0);
            style.spacing.button_padding = egui::vec2(12.0, 7.0);
            style.visuals.window_rounding = egui::Rounding::same(10.0);
        });

        // Верхняя панель со статусом
        egui::TopBottomPanel::top("status_bar").show(ctx, |ui| {
            ui.add_space(4.0);
            self.render_status_bar(ui);
            ui.add_space(4.0);
        });

        // Нижняя панель с логом
        egui::TopBottomPanel::bottom("log_panel").min_height(200.0).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Журнал операций");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Очистить").clicked() {
                        self.log_text.clear();
                        self.add_log("[ИНФО] Журнал очищен.");
                    }
                });
            });
            ui.add_space(4.0);

            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut self.log_text)
                            .code_editor()
                            .desired_width(f32::INFINITY)
                            .desired_rows(12)
                    );
                });
        });

        // Центральная панель с вкладками
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.heading(format!("DroidForge Plus v{}", env!("CARGO_PKG_VERSION")));
                ui.label(egui::RichText::new("ИНСТРУМЕНТЫ ДЛЯ ANDROID").small().color(egui::Color32::from_rgb(90, 190, 220)));
            });
            ui.label(
                egui::RichText::new("ADB, Fastboot и диагностика устройства в одном окне")
                    .color(egui::Color32::from_gray(160)),
            );
            ui.add_space(10.0);

            // Вкладки
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.current_tab, Tab::Adb, "📱 ADB");
                ui.selectable_value(&mut self.current_tab, Tab::Fastboot, "⚡ Загрузчик (Fastboot)");
            });

            ui.separator();
            ui.add_space(8.0);

            match self.current_tab {
                Tab::Adb => self.render_adb_tab(ui),
                Tab::Fastboot => self.render_fastboot_tab(ui),
            }
        });

        // Destructive Fastboot operations always require explicit confirmation.
        if let Some(action) = self.pending_action.clone() {
            let mut open = true;
            let mut confirm = false;
            let mut cancel = false;
            egui::Window::new("Подтверждение важного действия")
                .collapsible(false)
                .resizable(false)
                .open(&mut open)
                .show(ctx, |ui| {
                    match &action {
                        PendingAction::Flash { partition, image } => {
                            ui.colored_label(egui::Color32::YELLOW, "ВНИМАНИЕ: запись образа изменит раздел устройства.");
                            ui.label(format!("Раздел: {partition}"));
                            ui.label(format!("Образ: {image}"));
                            ui.label("Неподходящий образ может привести к невозможности загрузки.");
                        }
                        PendingAction::Erase { partition } => {
                            ui.colored_label(egui::Color32::RED, "ВНИМАНИЕ: содержимое раздела будет удалено.");
                            ui.label(format!("Раздел: {partition}"));
                        }
                        PendingAction::UnlockBootloader => {
                            ui.colored_label(egui::Color32::RED, "Разблокировка загрузчика обычно стирает все пользовательские данные.");
                            ui.label("Убедитесь, что резервная копия важных файлов создана.");
                        }
                        PendingAction::LockBootloader => {
                            ui.colored_label(egui::Color32::RED, "Повторная блокировка с неподходящей прошивкой может сделать устройство незагружаемым.");
                            ui.label("Продолжайте только если полностью понимаете последствия.");
                        }
                        PendingAction::RestoreBackup { directory } => {
                            ui.colored_label(egui::Color32::YELLOW, "Файлы будут скопированы в /sdcard/. Совпадающие файлы могут быть перезаписаны.");
                            ui.label(format!("Папка резервной копии: {directory}"));
                        }
                    }
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.button("Отмена").clicked() { cancel = true; }
                        if ui.button("Подтвердить операцию").clicked() { confirm = true; }
                    });
                });
            if confirm {
                self.pending_action = None;
                match action {
                    PendingAction::Flash { partition, image } => {
                        self.add_log(&format!("[ЗАГРУЗЧИК] Прошивка {partition} <- {image}"));
                        match android::flash_partition(&partition, &image) {
                            Ok(log) => self.add_log(&format!("[УСПЕХ] {log}")),
                            Err(e) => self.add_log(&format!("[ОШИБКА] {e}")),
                        }
                    }
                    PendingAction::Erase { partition } => {
                        self.add_log(&format!("[ЗАГРУЗЧИК] Стирание раздела {partition}"));
                        match android::erase_partition(&partition) {
                            Ok(log) => self.add_log(&format!("[УСПЕХ] {log}")),
                            Err(e) => self.add_log(&format!("[ОШИБКА] {e}")),
                        }
                    }
                    PendingAction::UnlockBootloader => {
                        self.add_log("[ЗАГРУЗЧИК] Разблокировка загрузчика...");
                        match android::unlock_bootloader() {
                            Ok(log) => self.add_log(&format!("[УСПЕХ] {log}")),
                            Err(e) => self.add_log(&format!("[ОШИБКА] {e}")),
                        }
                    }
                    PendingAction::LockBootloader => {
                        self.add_log("[ЗАГРУЗЧИК] Блокировка загрузчика...");
                        match android::lock_bootloader() {
                            Ok(log) => self.add_log(&format!("[УСПЕХ] {log}")),
                            Err(e) => self.add_log(&format!("[ОШИБКА] {e}")),
                        }
                    }
                    PendingAction::RestoreBackup { directory } => {
                        self.add_log(&format!("[ADB] Восстановление из {directory}"));
                        match android::restore_shared_storage(&directory) {
                            Ok(log) => self.add_log(&format!("[УСПЕХ] {log}")),
                            Err(e) => self.add_log(&format!("[ОШИБКА] {e}")),
                        }
                    }
                }
            } else if cancel || !open {
                self.pending_action = None;
                self.add_log("[ИНФО] Опасная операция отменена.");
            }
        }
    }
}

struct NativeWindowApp {
    window: Option<Arc<Window>>,
    instance: wgpu::Instance,
    surface: Option<wgpu::Surface<'static>>,
    surface_config: Option<wgpu::SurfaceConfiguration>,
    device: Option<wgpu::Device>,
    queue: Option<wgpu::Queue>,
    renderer: Option<egui_wgpu::Renderer>,
    egui_state: Option<egui_winit::State>,
    egui_ctx: egui::Context,
    app: DroidForgeApp,
}

impl Default for NativeWindowApp {
    fn default() -> Self {
        Self {
            window: None,
            instance: wgpu::Instance::default(),
            surface: None,
            surface_config: None,
            device: None,
            queue: None,
            renderer: None,
            egui_state: None,
            egui_ctx: egui::Context::default(),
            app: DroidForgeApp::default(),
        }
    }
}

impl NativeWindowApp {
    fn initialize_graphics(&mut self, window: Arc<Window>) -> Result<(), String> {
        let surface = self.instance.create_surface(window.clone())
            .map_err(|e| format!("Не удалось создать поверхность окна: {e}"))?;
        let adapter = pollster::block_on(self.instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            },
        )).ok_or_else(|| "Не найден совместимый графический адаптер.".to_string())?;

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("DroidForge Plus native WGPU device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_defaults(),
                memory_hints: wgpu::MemoryHints::MemoryUsage,
            },
            None,
        )).map_err(|e| format!("Не удалось создать графическое устройство: {e}"))?;

        let capabilities = surface.get_capabilities(&adapter);
        let format = capabilities.formats.iter().copied()
            .find(|f| *f == wgpu::TextureFormat::Bgra8Unorm || *f == wgpu::TextureFormat::Rgba8Unorm)
            .or_else(|| capabilities.formats.first().copied())
            .ok_or_else(|| "Видеодрайвер не предложил подходящий формат поверхности.".to_string())?;
        let alpha_mode = capabilities.alpha_modes.iter().copied()
            .find(|mode| *mode == wgpu::CompositeAlphaMode::Opaque)
            .unwrap_or(wgpu::CompositeAlphaMode::Auto);
        let size = window.inner_size();
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let renderer = egui_wgpu::Renderer::new(&device, format, None, 1, false);
        let egui_state = egui_winit::State::new(
            self.egui_ctx.clone(),
            egui::ViewportId::ROOT,
            window.as_ref(),
            Some(window.scale_factor() as f32),
            None,
            Some(device.limits().max_texture_dimension_2d as usize),
        );

        self.window = Some(window);
        self.surface = Some(surface);
        self.surface_config = Some(config);
        self.device = Some(device);
        self.queue = Some(queue);
        self.renderer = Some(renderer);
        self.egui_state = Some(egui_state);
        Ok(())
    }

    fn resize_surface(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 { return; }
        if let (Some(surface), Some(device), Some(config)) =
            (self.surface.as_ref(), self.device.as_ref(), self.surface_config.as_mut()) {
            config.width = width;
            config.height = height;
            surface.configure(device, config);
        }
    }

    fn render_frame(&mut self) {
        let (Some(window), Some(surface), Some(device), Some(queue), Some(renderer), Some(state), Some(config)) =
            (self.window.as_ref(), self.surface.as_ref(), self.device.as_ref(), self.queue.as_ref(),
             self.renderer.as_mut(), self.egui_state.as_mut(), self.surface_config.as_ref()) else {
            return;
        };

        let raw_input = state.take_egui_input(window);
        let output = self.egui_ctx.run(raw_input, |ctx| self.app.update_ui(ctx));
        state.handle_platform_output(window, output.platform_output);

        let frame = match surface.get_current_texture() {
            Ok(frame) => frame,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                self.resize_surface(config.width, config.height);
                return;
            }
            Err(wgpu::SurfaceError::Timeout) => return,
            Err(wgpu::SurfaceError::OutOfMemory) => {
                eprintln!("WGPU: недостаточно видеопамяти для отображения окна.");
                return;
            }
        };
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let pixels_per_point = window.scale_factor() as f32 * self.egui_ctx.zoom_factor();
        let screen = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [config.width, config.height],
            pixels_per_point,
        };
        let paint_jobs = self.egui_ctx.tessellate(output.shapes, pixels_per_point);
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("DroidForge Plus UI encoder"),
        });

        for (id, image_delta) in &output.textures_delta.set {
            renderer.update_texture(device, queue, *id, image_delta);
        }
        let user_cmd_buffers = renderer.update_buffers(device, queue, &mut encoder, &paint_jobs, &screen);
        {
            let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("DroidForge Plus opaque UI pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 15.0 / 255.0,
                            g: 18.0 / 255.0,
                            b: 25.0 / 255.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            renderer.render(&mut pass.forget_lifetime(), &paint_jobs, &screen);
        }
        queue.submit(user_cmd_buffers.into_iter().chain(std::iter::once(encoder.finish())));
        frame.present();

        for id in &output.textures_delta.free {
            renderer.free_texture(id);
        }
    }
}

impl ApplicationHandler for NativeWindowApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() { return; }
        let attributes = Window::default_attributes()
            .with_title("DroidForge Plus")
            .with_inner_size(winit::dpi::LogicalSize::new(1100.0, 800.0))
            .with_min_inner_size(winit::dpi::LogicalSize::new(760.0, 560.0))
            .with_transparent(false);
        match event_loop.create_window(attributes) {
            Ok(window) => {
                let window = Arc::new(window);
                if let Err(error) = self.initialize_graphics(window.clone()) {
                    eprintln!("DroidForge Plus: {error}");
                    event_loop.exit();
                    return;
                }
                window.request_redraw();
            }
            Err(error) => {
                eprintln!("Не удалось создать нативное окно Windows: {error}");
                event_loop.exit();
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, window_id: WindowId, event: WindowEvent) {
        let Some(window) = self.window.as_ref() else { return; };
        if window.id() != window_id { return; }

        if let Some(state) = self.egui_state.as_mut() {
            let response = state.on_window_event(window, &event);
            if response.repaint { window.request_redraw(); }
        }

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => self.resize_surface(size.width, size.height),
            WindowEvent::ScaleFactorChanged { .. } => {
                let size = window.inner_size();
                self.resize_surface(size.width, size.height);
            }
            WindowEvent::RedrawRequested => self.render_frame(),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        event_loop.set_control_flow(ControlFlow::WaitUntil(
            std::time::Instant::now() + std::time::Duration::from_millis(100)
        ));
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoop::new()?;
    let mut app = NativeWindowApp::default();
    event_loop.run_app(&mut app)?;
    Ok(())
}
