use gtk::{Application, Button, prelude::*};
use hyprparser::parse_config;
use std::{cell::RefCell, env, fs, path::Path, path::PathBuf, rc::Rc};

mod gui;

const CONFIG_PATH: &str = ".config/hypr/hyprland.conf";
const BACKUP_SUFFIX: &str = "-bak";

fn main() {
    let app = Application::builder().application_id("hyprviz").build();

    app.connect_activate(build_ui);
    app.run();
}

fn build_ui(app: &Application) {
    let gui = Rc::new(RefCell::new(gui::ConfigGUI::new(app)));
    gui::ConfigGUI::setup_config_buttons(gui.clone());

    let config_path_full = get_config_path();

    if !config_path_full.exists() {
        gui.borrow_mut().custom_error_popup_critical(
            "File not found",
            &format!("File not found: ~/{CONFIG_PATH}"),
            true,
        );
    } else {
        let config_str = match fs::read_to_string(&config_path_full) {
            Ok(s) => s,
            Err(e) => {
                gui.borrow_mut().custom_error_popup_critical(
                    "Reading failed",
                    &format!("Failed to read the configuration file: {e}"),
                    true,
                );
                String::new()
            }
        };
        let parsed_config = parse_config(&config_str);
        gui.borrow_mut().load_config(&parsed_config);

        let gui_clone = gui.clone();
        gui.borrow().save_button.connect_clicked(move |_| {
            save_config_file(gui_clone.clone());
        });

        let undo_button = Button::with_label("Undo Changes");
        let copy_button = Button::with_label("Copyright");

        let gui_clone = gui.clone();
        undo_button.connect_clicked(move |_| {
            undo_changes(gui_clone.clone());
        });

        let gui_clone = gui.clone();
        copy_button.connect_clicked(move |_| {
            gui_clone.borrow_mut().custom_info_popup(
                "idk what to put here",
                "This program is free software; you can redistribute it and/or
modify it under the terms of the GNU General Public License
as published by the Free Software Foundation, version 2 of
the License.
This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU General Public License for more details.
You should have received a copy of the GNU General Public License
along with this program; if not, see
<https://www.gnu.org/licenses/old-licenses/gpl-2.0>.",
                true,
            );
        });

        if let Some(gear_menu_box) = gui.borrow().gear_menu.borrow().child() {
            if let Some(box_widget) = gear_menu_box.downcast_ref::<gtk::Box>() {
                box_widget.append(&undo_button);
                box_widget.append(&copy_button);
            }
        }
    }

    gui.borrow().window.present();
}

fn save_config_file(gui: Rc<RefCell<gui::ConfigGUI>>) {
    let mut gui_ref = gui.borrow_mut();
    let path = get_config_path();
    let backup_path = path.with_file_name(format!(
        "{}{}",
        path.file_name().unwrap().to_str().unwrap(),
        BACKUP_SUFFIX
    ));

    let config_str = match fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => {
            gui_ref.custom_error_popup_critical(
                "Reading failed",
                &format!("Failed to read the configuration file: {e}"),
                true,
            );
            return;
        }
    };

    let mut parsed_config = parse_config(&config_str);
    let changes = gui_ref.get_changes();

    if !changes.borrow().is_empty() {
        if !backup_path.exists() {
            if let Err(e) = fs::copy(&path, &backup_path) {
                gui_ref.custom_error_popup(
                    "Backup failed",
                    &format!("Failed to create backup: {e}"),
                    true,
                );
                return;
            }
        }

        gui_ref.apply_changes(&mut parsed_config);

        let updated_config_str = parsed_config.to_string();

        match fs::write(&path, updated_config_str) {
            Ok(_) => println!("Configuration saved to: ~/{CONFIG_PATH}"),
            Err(e) => {
                gui_ref.custom_error_popup(
                    "Saving failed",
                    &format!("Failed to save the configuration: {e}"),
                    true,
                );
            }
        }
    } else {
        gui_ref.custom_info_popup("Saving failed", "No changes to save.", true);
    }
}

fn undo_changes(gui: Rc<RefCell<gui::ConfigGUI>>) {
    let mut gui_ref = gui.borrow_mut();
    let path = get_config_path();
    let backup_path = path.with_file_name(format!(
        "{}{}",
        path.file_name().unwrap().to_str().unwrap(),
        BACKUP_SUFFIX
    ));

    if backup_path.exists() {
        match fs::copy(&backup_path, &path) {
            Ok(_) => {
                println!("Configuration restored from backup");
                if let Ok(config_str) = fs::read_to_string(&path) {
                    let parsed_config = parse_config(&config_str);
                    gui_ref.load_config(&parsed_config);

                    gui_ref.get_changes().borrow_mut().clear();

                    if let Err(e) = fs::remove_file(&backup_path) {
                        gui_ref.custom_error_popup(
                            "Backup Deletion Failed",
                            &format!("Failed to delete the backup file: {e}"),
                            true,
                        );
                    } else {
                        gui_ref.custom_info_popup(
                            "Undo Successful",
                            "Configuration restored from backup and backup file deleted.",
                            true,
                        );
                    }
                } else {
                    gui_ref.custom_error_popup(
                        "Reload Failed",
                        "Failed to reload the configuration after undo.",
                        true,
                    );
                }
            }
            Err(e) => {
                gui_ref.custom_error_popup(
                    "Undo Failed",
                    &format!("Failed to restore from backup: {e}"),
                    true,
                );
            }
        }
    } else {
        gui_ref.custom_error_popup(
            "Undo Failed",
            "No backup file found. Save changes at least once to create a backup.",
            true,
        );
    }
}

fn get_config_path() -> PathBuf {
    Path::new(&env::var("HOME").unwrap_or_else(|_| ".".to_string())).join(CONFIG_PATH)
}
