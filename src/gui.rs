use gtk::{
    AlertDialog, Application, ApplicationWindow, Box, Button, ColorDialog, ColorDialogButton,
    DropDown, Entry, FileDialog, Frame, HeaderBar, Image, Label, Orientation, Popover,
    ScrolledWindow, SpinButton, Stack, StackSidebar, StringList, Switch, Widget, gdk, glib,
    prelude::*,
};

use hyprparser::HyprlandConfig;
use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;

fn add_dropdown_option(
    container: &Box,
    options: &mut HashMap<String, Widget>,
    name: &str,
    label: &str,
    description: &str,
    items: &[&str],
) {
    let hbox = Box::new(Orientation::Horizontal, 10);
    hbox.set_margin_start(10);
    hbox.set_margin_end(10);
    hbox.set_margin_top(5);
    hbox.set_margin_bottom(5);

    let label_box = Box::new(Orientation::Horizontal, 5);
    label_box.set_hexpand(true);

    let label_widget = Label::new(Some(label));
    label_widget.set_halign(gtk::Align::Start);

    let tooltip_button = Button::new();
    let question_mark_icon = Image::from_icon_name("dialog-question-symbolic");
    tooltip_button.set_child(Some(&question_mark_icon));
    tooltip_button.set_has_frame(false);

    let popover = Popover::new();
    let description_label = Label::new(Some(description));
    description_label.set_margin_top(5);
    description_label.set_margin_bottom(5);
    description_label.set_margin_start(5);
    description_label.set_margin_end(5);
    popover.set_child(Some(&description_label));
    popover.set_position(gtk::PositionType::Right);

    tooltip_button.connect_clicked(move |button| {
        popover.set_parent(button);
        popover.popup();
    });

    label_box.append(&label_widget);
    label_box.append(&tooltip_button);

    let string_list = StringList::new(items);
    let dropdown = DropDown::new(Some(string_list), None::<gtk::Expression>);
    dropdown.set_halign(gtk::Align::End);
    dropdown.set_width_request(100);

    hbox.append(&label_box);
    hbox.append(&dropdown);

    container.append(&hbox);

    options.insert(name.to_string(), dropdown.upcast());
}

pub struct ConfigGUI {
    pub window: ApplicationWindow,
    config_widgets: HashMap<String, ConfigWidget>,
    pub save_button: Button,
    content_box: Box,
    changed_options: Rc<RefCell<HashMap<(String, String), String>>>,
    stack: Stack,
    sidebar: StackSidebar,
    load_config_button: Button,
    save_config_button: Button,
    pub gear_menu: Rc<RefCell<Popover>>,
}

impl ConfigGUI {
    pub fn new(app: &Application) -> Self {
        let window = ApplicationWindow::builder()
            .application(app)
            .default_width(1000)
            .default_height(600)
            .build();

        let header_bar = HeaderBar::builder()
            .show_title_buttons(false)
            .title_widget(&gtk::Label::new(Some("Hyprland Configuration")))
            .build();

        let gear_button = Button::from_icon_name("emblem-system-symbolic");
        header_bar.pack_start(&gear_button);

        let gear_menu = Rc::new(RefCell::new(Popover::new()));
        gear_menu.borrow().set_parent(&gear_button);

        let gear_menu_box = Box::new(Orientation::Vertical, 5);
        gear_menu_box.set_margin_top(5);
        gear_menu_box.set_margin_bottom(5);
        gear_menu_box.set_margin_start(5);
        gear_menu_box.set_margin_end(5);

        let save_config_button = Button::with_label("Save HyprViz Config");
        let load_config_button = Button::with_label("Load HyprViz Config");

        gear_menu_box.append(&load_config_button);
        gear_menu_box.append(&save_config_button);

        gear_menu.borrow().set_child(Some(&gear_menu_box));

        let gear_menu_clone = gear_menu.clone();
        gear_button.connect_clicked(move |_| {
            gear_menu_clone.borrow().popup();
        });

        let tooltip_button = Button::new();
        let question_mark_icon = Image::from_icon_name("dialog-question-symbolic");
        tooltip_button.set_child(Some(&question_mark_icon));
        tooltip_button.set_has_frame(false);
        header_bar.pack_start(&tooltip_button);

        let popover = Popover::new();
        let tooltip_text = "The save button saves the options that you chose in the gui and exports it to json format, likewise the load button loads these saved options from the exported json file; automatically filling in the options in the gui with the specified ones in the json file, clicking save to apply these changes is still necessary though.";
        let tooltip_label = Label::new(Some(tooltip_text));
        tooltip_label.set_margin_top(5);
        tooltip_label.set_margin_bottom(5);
        tooltip_label.set_margin_start(5);
        tooltip_label.set_margin_end(5);
        tooltip_label.set_wrap(true);
        tooltip_label.set_max_width_chars(50);
        popover.set_child(Some(&tooltip_label));

        tooltip_button.connect_clicked(move |button| {
            popover.set_parent(button);
            popover.popup();
        });

        let save_button = Button::with_label("Save");
        header_bar.pack_end(&save_button);

        window.set_titlebar(Some(&header_bar));

        let main_box = Box::new(Orientation::Vertical, 0);

        let content_box = Box::new(Orientation::Horizontal, 0);
        main_box.append(&content_box);

        window.set_child(Some(&main_box));

        let config_widgets = HashMap::new();

        let stack = Stack::new();

        let sidebar = StackSidebar::new();
        sidebar.set_stack(&stack);
        sidebar.set_width_request(200);

        ConfigGUI {
            window,
            config_widgets,
            save_button,
            content_box,
            changed_options: Rc::new(RefCell::new(HashMap::new())),
            stack,
            sidebar,
            load_config_button,
            save_config_button,
            gear_menu,
        }
    }

    pub fn setup_config_buttons(gui: Rc<RefCell<ConfigGUI>>) {
        // LOAD CONFIG BUTTON
        {
            let gui_clone = Rc::clone(&gui);
            gui.borrow().load_config_button.connect_clicked(move |_| {
                let gui = Rc::clone(&gui_clone);

                // Borrow GUI here — it's dropped at the end of this block
                let window = {
                    let g = gui.borrow();
                    g.window.clone()
                };

                glib::MainContext::default().spawn_local(async move {
                    let dialog = FileDialog::builder()
                        .title("Load HyprViz Config")
                        .accept_label("Open")
                        .build();

                    if let Ok(file) = dialog.open_future(Some(&window)).await {
                        if let Some(path) = file.path() {
                            gui.borrow_mut().load_hyprviz_config(&path);
                        }
                    }
                });
            });
        }

        // SAVE CONFIG BUTTON
        {
            let gui_clone = Rc::clone(&gui);
            gui.borrow().save_config_button.connect_clicked(move |_| {
                let gui = Rc::clone(&gui_clone);

                let window = {
                    let g = gui.borrow();
                    g.window.clone()
                };

                glib::MainContext::default().spawn_local(async move {
                    let dialog = FileDialog::builder()
                        .title("Save HyprViz Config")
                        .initial_name("hyprviz_config.json")
                        .accept_label("Save")
                        .build();

                    if let Ok(file) = dialog.save_future(Some(&window)).await {
                        if let Some(path) = file.path() {
                            gui.borrow_mut().save_hyprviz_config(&path);
                        }
                    }
                });
            });
        }
    }


    fn load_hyprviz_config(&mut self, path: &PathBuf) {
        match fs::read_to_string(path) {
            Ok(content) => {
                if let Ok(config) = serde_json::from_str::<HashMap<String, String>>(&content) {
                    for (key, value) in config {
                        let parts: Vec<&str> = key.split(':').collect();
                        if parts.len() >= 2 {
                            let category = parts[0].to_string();
                            let name = parts[1..].join(":");
                            if let Some(widget) = self.config_widgets.get(&category) {
                                if let Some(option_widget) = widget.options.get(&name) {
                                    self.set_widget_value(option_widget, &value);
                                    self.changed_options
                                        .borrow_mut()
                                        .insert((category, name), value);
                                }
                            }
                        }
                    }
                    self.custom_info_popup(
                        "Config Loaded",
                        "HyprViz configuration loaded successfully.",
                        false,
                    );
                } else {
                    self.custom_error_popup(
                        "Invalid Config",
                        "Failed to parse the configuration file.",
                        false,
                    );
                }
            }
            Err(e) => {
                self.custom_error_popup(
                    "Loading Failed",
                    &format!("Failed to read the configuration file: {e}"),
                    false,
                );
            }
        }
    }

    fn save_hyprviz_config(&mut self, path: &PathBuf) {
        let config: HashMap<String, String> = self
            .changed_options
            .borrow()
            .iter()
            .map(|((category, name), value)| (format!("{category}:{name}"), value.clone()))
            .collect();

        match serde_json::to_string_pretty(&config) {
            Ok(json) => match fs::write(path, json) {
                Ok(_) => {
                    self.custom_info_popup(
                        "Config Saved",
                        "HyprViz configuration saved successfully.",
                        false,
                    );
                }
                Err(e) => {
                    self.custom_error_popup(
                        "Saving Failed",
                        &format!("Failed to write the configuration file: {e}"),
                        false,
                    );
                }
            },
            Err(e) => {
                self.custom_error_popup(
                    "Serialization Failed",
                    &format!("Failed to serialize the configuration: {e}"),
                    false,
                );
            }
        }
    }

    fn set_widget_value(&self, widget: &Widget, value: &str) {
        if let Some(spin_button) = widget.downcast_ref::<SpinButton>() {
            if let Ok(float_value) = value.parse::<f64>() {
                spin_button.set_value(float_value);
            }
        } else if let Some(entry) = widget.downcast_ref::<Entry>() {
            entry.set_text(value);
        } else if let Some(switch) = widget.downcast_ref::<Switch>() {
            switch.set_active(value == "true" || value == "1");
        } else if let Some(color_button) = widget.downcast_ref::<ColorDialogButton>() {
            let dummy_config = HyprlandConfig::new();
            if let Some((red, green, blue, alpha)) = dummy_config.parse_color(value) {
                color_button.set_rgba(&gdk::RGBA::new(red, green, blue, alpha));
            }
        } else if let Some(dropdown) = widget.downcast_ref::<DropDown>() {
            let model = dropdown.model().unwrap();
            for i in 0..model.n_items() {
                if let Some(item) = model.item(i) {
                    if let Some(string_object) = item.downcast_ref::<gtk::StringObject>() {
                        if string_object.string() == value {
                            dropdown.set_selected(i);
                            break;
                        }
                    }
                }
            }
        }
    }

    pub fn custom_info_popup(&mut self, title: &str, text: &str, modal: bool) {
        let dialog = AlertDialog::builder()
            .message(title)
            .detail(text)
            .buttons(&["OK"][..])
            .modal(modal)
            .build();
        dialog.show(None::<&gtk::Window>);
    }

    pub fn custom_error_popup(&mut self, title: &str, text: &str, modal: bool) {
        let dialog = AlertDialog::builder()
            .message(title)
            .detail(text)
            .buttons(&["OK"][..])
            .modal(modal)
            .build();
        dialog.show(None::<&gtk::Window>);
    }

    pub fn custom_error_popup_critical(&mut self, title: &str, text: &str, modal: bool) {
        let dialog = AlertDialog::builder()
            .message(title)
            .detail(text)
            .buttons(&["OK"][..])
            .modal(modal)
            .build();
        dialog.choose(
            None::<&gtk::Window>,
            None::<&gio::Cancellable>,
            move |_res: Result<i32, _>| {
                std::process::exit(1);
            },
        );
    }

    pub fn load_config(&mut self, config: &HyprlandConfig) {
        self.config_widgets.clear();
        self.content_box.set_visible(true);

        while let Some(child) = self.stack.first_child() {
            self.stack.remove(&child);
        }

        while let Some(child) = self.content_box.first_child() {
            self.content_box.remove(&child);
        }

        self.sidebar = StackSidebar::new();
        self.sidebar.set_stack(&self.stack);
        self.sidebar.set_width_request(200);

        self.content_box.append(&self.sidebar);
        self.content_box.append(&self.stack);

        self.stack.connect_visible_child_notify(move |stack| {
            if let Some(child) = stack.visible_child() {
                if let Some(scrolled_window) = child.downcast_ref::<ScrolledWindow>() {
                    let adj = scrolled_window.vadjustment();
                    adj.set_value(adj.lower());
                }
            }
        });

        let categories = [
            ("General", "general"),
            ("Decoration", "decoration"),
            ("Animations", "animations"),
            ("Input", "input"),
            ("Gestures", "gestures"),
            ("Misc", "misc"),
            ("Binds", "binds"),
            ("Group", "group"),
            ("Layouts", "layouts"),
            ("XWayland", "xwayland"),
            ("OpenGL", "opengl"),
            ("Render", "render"),
            ("Cursor", "cursor"),
            ("Ecosystem", "ecosystem"),
            ("Experimental", "experimental"),
            ("Debug", "debug"),
        ];

        for (display_name, category) in &categories {
            let widget = ConfigWidget::new(category);
            self.stack
                .add_titled(&widget.scrolled_window, Some(category), display_name);
            self.config_widgets.insert(category.to_string(), widget);
        }

        for (_, category) in &categories {
            if let Some(widget) = self.config_widgets.get(*category) {
                widget.load_config(config, category, self.changed_options.clone());
            }
        }

        self.changed_options.borrow_mut().clear();
    }

    pub fn get_changes(&self) -> Rc<RefCell<HashMap<(String, String), String>>> {
        self.changed_options.clone()
    }

    pub fn apply_changes(&self, config: &mut HyprlandConfig) {
        let changes = self.changed_options.borrow();
        for (category, widget) in &self.config_widgets {
            for (name, widget) in &widget.options {
                if let Some(value) = changes.get(&(category.to_string(), name.to_string())) {
                    let formatted_value =
                        if let Some(color_button) = widget.downcast_ref::<ColorDialogButton>() {
                            let rgba = color_button.rgba();
                            format!(
                                "rgba({:02X}{:02X}{:02X}{:02X})",
                                (rgba.red() * 255.0) as u8,
                                (rgba.green() * 255.0) as u8,
                                (rgba.blue() * 255.0) as u8,
                                (rgba.alpha() * 255.0) as u8
                            )
                        } else {
                            value.clone()
                        };

                    if !formatted_value.is_empty() {
                        if category == "layouts" {
                            let parts: Vec<&str> = name.split(':').collect();
                            if parts.len() == 2 {
                                config.add_entry(
                                    parts[0],
                                    &format!("{} = {}", parts[1], formatted_value),
                                );
                            }
                        } else if name.contains(':') {
                            let parts: Vec<&str> = name.split(':').collect();
                            if parts.len() == 2 {
                                config.add_entry(
                                    &format!("{}.{}", category, parts[0]),
                                    &format!("{} = {}", parts[1], formatted_value),
                                );
                            }
                        } else {
                            config.add_entry(category, &format!("{name} = {formatted_value}"));
                        }
                    }
                }
            }
        }
    }
}

fn get_option_limits(name: &str, description: &str) -> (f64, f64, f64) {
    match name {
        "border_size" => (0.0, 10.0, 1.0),
        "gaps_in" | "gaps_out" | "gaps_workspaces" | "float_gaps" => (0.0, 500.0, 1.0),
        "resize_corner" => (0.0, 4.0, 1.0),
        "rounding" => (0.0, 500.0, 1.0),
        "rounding_power" => (2.0, 10.0, 0.1),
        "active_opacity" | "inactive_opacity" | "fullscreen_opacity" => (0.0, 1.0, 0.1),
        "shadow:range" => (0.0, 500.0, 1.0),
        "shadow:render_power" => (1.0, 4.0, 1.0),
        "shadow:scale" => (0.0, 1.0, 0.1),
        "dim_strength" | "dim_special" | "dim_around" => (0.0, 1.0, 0.1),
        "blur:size" => (1.0, 20.0, 1.0),
        "blur:passes" => (1.0, 10.0, 1.0),
        "blur:noise" => (0.0, 1.0, 0.01),
        "blur:contrast" => (0.0, 2.0, 0.1),
        "blur:brightness" => (0.0, 2.0, 0.1),
        "blur:vibrancy" | "blur:vibrancy_darkness" => (0.0, 1.0, 0.1),
        "blur:popups_ignorealpha" => (0.0, 1.0, 0.1),
        "touchpad:drag_3fg" => (0.0, 2.0, 1.0),
        "sensitivity" => (-1.0, 1.0, 0.1),
        "scroll_button" => (0.0, 9.0, 1.0),
        "scroll_factor" => (0.1, 10.0, 0.1),
        "follow_mouse" => (0.0, 3.0, 1.0),
        "float_switch_override_focus" => (0.0, 2.0, 1.0),
        "workspace_swipe_fingers" => (2.0, 5.0, 1.0),
        "workspace_swipe_distance" => (100.0, 500.0, 10.0),
        "workspace_swipe_min_speed_to_force" => (0.0, 100.0, 1.0),
        "workspace_swipe_cancel_ratio" => (0.0, 1.0, 0.1),
        "workspace_swipe_direction_lock_threshold" => (0.0, 50.0, 1.0),
        "drag_into_group" => (0.0, 2.0, 1.0),
        "force_default_wallpaper" => (-1.0, 2.0, 1.0),
        "vrr" => (0.0, 2.0, 1.0),
        "render_ahead_safezone" => (0.0, 10.0, 1.0),
        "new_window_takes_over_fullscreen" => (0.0, 2.0, 1.0),
        "initial_workspace_tracking" => (0.0, 2.0, 1.0),
        "render_unfocused_fps" => (1.0, 60.0, 1.0),
        "scroll_event_delay" => (0.0, 1000.0, 10.0),
        "workspace_center_on" => (0.0, 1.0, 1.0),
        "focus_preferred_method" => (0.0, 1.0, 1.0),
        "force_introspection" => (0.0, 2.0, 1.0),
        "explicit_sync" | "explicit_sync_kms" => (0.0, 2.0, 1.0),
        "min_refresh_rate" => (1.0, 240.0, 1.0),
        "hotspot_padding" => (0.0, 10.0, 1.0),
        "inactive_timeout" => (0.0, 60.0, 1.0),
        "zoom_factor" => (1.0, 5.0, 0.1),
        "damage_tracking" => (0.0, 2.0, 1.0),
        "watchdog_timeout" => (0.0, 60.0, 1.0),
        "error_limit" => (1.0, 100.0, 1.0),
        "error_position" => (0.0, 1.0, 1.0),
        "repeat_rate" => (1.0, 100.0, 1.0),
        "repeat_delay" => (100.0, 2000.0, 100.0),
        "touchpad:scroll_factor" => (0.1, 10.0, 0.1),
        "tablet:transform" => (0.0, 7.0, 1.0),
        "off_window_axis_events" => (0.0, 3.0, 1.0),
        "emulate_discrete_scroll" => (0.0, 2.0, 1.0),
        "focus_on_close" => (0.0, 1.0, 1.0),
        "groupbar:font_size" => (6.0, 32.0, 1.0),
        "groupbar:height" => (10.0, 50.0, 1.0),
        "groupbar:priority" => (0.0, 10.0, 1.0),
        "manual_crash" => (0.0, 1.0, 1.0),
        _ => {
            if description.contains("[0.0 - 1.0]") {
                (0.0, 1.0, 0.1)
            } else if description.contains("[0/1]") {
                (0.0, 1.0, 1.0)
            } else if description.contains("[0/1/2]") {
                (0.0, 2.0, 1.0)
            } else if name.contains("opacity") || name.contains("ratio") {
                (0.0, 1.0, 0.1)
            } else {
                (0.0, 50.0, 1.0)
            }
        }
    }
}

pub struct ConfigWidget {
    options: HashMap<String, Widget>,
    scrolled_window: ScrolledWindow,
}

impl ConfigWidget {
    fn new(category: &str) -> Self {
        let scrolled_window = ScrolledWindow::new();
        scrolled_window.set_vexpand(false);
        scrolled_window.set_propagate_natural_height(true);

        let container = Box::new(Orientation::Vertical, 0);
        container.set_margin_start(20);
        container.set_margin_end(20);
        container.set_margin_top(20);
        container.set_margin_bottom(20);

        scrolled_window.set_child(Some(&container));

        let mut options = HashMap::new();

        let first_section = Rc::new(RefCell::new(true));

        match category {
            "general" => {
                Self::add_section(
                    &container,
                    "General Settings",
                    "Configure general behavior.",
                    first_section.clone(),
                );

                Self::add_section(
                    &container,
                    "Layout",
                    "Choose the default layout.",
                    first_section.clone(),
                );
                add_dropdown_option(
                    &container,
                    &mut options,
                    "layout",
                    "Layout",
                    "which layout to use.",
                    &["dwindle", "master"],
                );
                Self::add_section(
                    &container,
                    "Gaps",
                    "Change gaps in & out, workspaces.",
                    first_section.clone(),
                );
                Self::add_int_option(
                    &container,
                    &mut options,
                    "gaps_in",
                    "Gaps In",
                    "gaps between windows, also supports css style gaps (top, right, bottom, left -> 5,10,15,20)",
                );
                Self::add_int_option(
                    &container,
                    &mut options,
                    "gaps_out",
                    "Gaps Out",
                    "gaps between windows and monitor edges, also supports css style gaps (top, right, bottom, left -> 5,10,15,20)",
                );
                Self::add_int_option(
                    &container,
                    &mut options,
                    "float_gaps",
                    "Fload gaps",
                    "gaps between windows and monitor edges for floating windows, also supports css style gaps (top, right, bottom, left -> 5 10 15 20). -1 means default",
                );
                Self::add_int_option(
                    &container,
                    &mut options,
                    "gaps_workspaces",
                    "Gaps Workspaces",
                    "gaps between workspaces. Stacks with gaps_out.",
                );

                Self::add_section(
                    &container,
                    "Borders",
                    "Size, resize, floating...",
                    first_section.clone(),
                );
                Self::add_int_option(
                    &container,
                    &mut options,
                    "border_size",
                    "Border Size",
                    "size of the border around windows",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "no_border_on_floating",
                    "No Border on Floating",
                    "disable borders for floating windows",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "resize_on_border",
                    "Resize on Border",
                    "enables resizing windows by clicking and dragging on borders and gaps",
                );
                Self::add_int_option(
                    &container,
                    &mut options,
                    "resize_corner",
                    "Resize Corner",
                    "force floating windows to use a specific corner when being resized (1-4 going clockwise from top left, 0 to disable)",
                );
                Self::add_int_option(
                    &container,
                    &mut options,
                    "extend_border_grab_area",
                    "Extend Border Grab Area",
                    "extends the area around the border where you can click and drag on, only used when general:resize_on_border is on.",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "hover_icon_on_border",
                    "Hover Icon on Border",
                    "show a cursor icon when hovering over borders, only used when general:resize_on_border is on.",
                );

                Self::add_section(
                    &container,
                    "Snap",
                    "Configure snap behavior.",
                    first_section.clone(),
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "snap:enabled",
                    "Enabled",
                    "enable snapping for floating windows",
                );
                Self::add_int_option(
                    &container,
                    &mut options,
                    "snap:window_gap",
                    "Window Gap",
                    "minimum gap in pixels between windows before snapping",
                );
                Self::add_int_option(
                    &container,
                    &mut options,
                    "snap:monitor_gap",
                    "Monitor Gap",
                    "minimum gap in pixels between window and monitor edges before snapping",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "snap:border_overlap",
                    "Border Overlap",
                    "if true, windows snap such that only one border’s worth of space is between them",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "snap:",
                    "respect_gaps",
                    "if true, snapping will respect gaps between windows(set in general:gaps_in)",
                );

                Self::add_section(
                    &container,
                    "Other stuff",
                    "Some other settings.",
                    first_section.clone(),
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "no_focus_fallback",
                    "No Focus Fallback",
                    "if true, will not fall back to the next available window when moving focus in a direction where no window was found.",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "allow_tearing",
                    "Allow Tearing",
                    "master switch for allowing tearing to occur. See the Hyprland's Tearing page.",
                );

                Self::add_section(
                    &container,
                    "Colors",
                    "Change borders colors.",
                    first_section.clone(),
                );
                Self::add_color_option(
                    &container,
                    &mut options,
                    "col.inactive_border",
                    "Inactive Border Color",
                    "border color for inactive windows",
                );
                Self::add_color_option(
                    &container,
                    &mut options,
                    "col.active_border",
                    "Active Border Color",
                    "border color for the active window",
                );
                Self::add_color_option(
                    &container,
                    &mut options,
                    "col.nogroup_border",
                    "No Group Border Color",
                    "inactive border color for window that cannot be added to a group (see denywindowfromgroup dispatcher)",
                );
                Self::add_color_option(
                    &container,
                    &mut options,
                    "col.nogroup_border_active",
                    "No Group Active Border Color",
                    "active border color for window that cannot be added to a group",
                );
            }
            "decoration" => {
                Self::add_section(
                    &container,
                    "Window Decoration",
                    "Configure window appearance.",
                    first_section.clone(),
                );
                Self::add_int_option(
                    &container,
                    &mut options,
                    "rounding",
                    "Rounding",
                    "rounded corners' radius (in layout px)",
                );
                Self::add_int_option(
                    &container,
                    &mut options,
                    "rounding_power",
                    "Rounding Power",
                    "adjusts the curve used for rounding corners, larger is smoother, 2.0 is a circle, 4.0 is a squircle. [2.0 - 10.0]",
                );
                Self::add_float_option(
                    &container,
                    &mut options,
                    "active_opacity",
                    "Active Opacity",
                    "opacity of active windows. [0.0 - 1.0]",
                );
                Self::add_float_option(
                    &container,
                    &mut options,
                    "inactive_opacity",
                    "Inactive Opacity",
                    "opacity of inactive windows. [0.0 - 1.0]",
                );
                Self::add_float_option(
                    &container,
                    &mut options,
                    "fullscreen_opacity",
                    "Fullscreen Opacity",
                    "opacity of fullscreen windows. [0.0 - 1.0]",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "dim_inactive",
                    "Dim Inactive",
                    "enables dimming of inactive windows",
                );
                Self::add_float_option(
                    &container,
                    &mut options,
                    "dim_strength",
                    "Dim Strength",
                    "how much inactive windows should be dimmed [0.0 - 1.0]",
                );
                Self::add_float_option(
                    &container,
                    &mut options,
                    "dim_special",
                    "Dim Special",
                    "how much to dim the rest of the screen by when a special workspace is open. [0.0 - 1.0]",
                );
                Self::add_float_option(
                    &container,
                    &mut options,
                    "dim_around",
                    "Dim Around",
                    "how much the dimaround window rule should dim by. [0.0 - 1.0]",
                );
                Self::add_string_option(
                    &container,
                    &mut options,
                    "screen_shader",
                    "Screen Shader",
                    "a path to a custom shader to be applied at the end of rendering. See examples/screenShader.frag for an example.",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "border_part_of_window",
                    "Border Part of Window",
                    "whether the window border should be a part of the window",
                );

                Self::add_section(
                    &container,
                    "Blur",
                    "Configure blur settings.",
                    first_section.clone(),
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "blur:enabled",
                    "Blur Enabled",
                    "enable kawase window background blur",
                );
                Self::add_int_option(
                    &container,
                    &mut options,
                    "blur:size",
                    "Blur Size",
                    "blur size (distance)",
                );
                Self::add_int_option(
                    &container,
                    &mut options,
                    "blur:passes",
                    "Blur Passes",
                    "the amount of passes to perform",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "blur:ignore_opacity",
                    "Blur Ignore Opacity",
                    "make the blur layer ignore the opacity of the window",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "blur:new_optimizations",
                    "Blur New Optimizations",
                    "whether to enable further optimizations to the blur. Recommended to leave on, as it will massively improve performance.",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "blur:xray",
                    "Blur X-Ray",
                    "if enabled, floating windows will ignore tiled windows in their blur. Only available if blur_new_optimizations is true. Will reduce overhead on floating blur significantly.",
                );
                Self::add_float_option(
                    &container,
                    &mut options,
                    "blur:noise",
                    "Blur Noise",
                    "how much noise to apply. [0.0 - 1.0]",
                );
                Self::add_float_option(
                    &container,
                    &mut options,
                    "blur:contrast",
                    "Blur Contrast",
                    "contrast modulation for blur. [0.0 - 2.0]",
                );
                Self::add_float_option(
                    &container,
                    &mut options,
                    "blur:brightness",
                    "Blur Brightness",
                    "brightness modulation for blur. [0.0 - 2.0]",
                );
                Self::add_float_option(
                    &container,
                    &mut options,
                    "blur:vibrancy",
                    "Blur Vibrancy",
                    "Increase saturation of blurred colors. [0.0 - 1.0]",
                );
                Self::add_float_option(
                    &container,
                    &mut options,
                    "blur:vibrancy_darkness",
                    "Blur Vibrancy Darkness",
                    "How strong the effect of vibrancy is on dark areas . [0.0 - 1.0]",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "blur:special",
                    "Blur Special",
                    "whether to blur behind the special workspace (note: expensive)",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "blur:popups",
                    "Blur Popups",
                    "whether to blur popups (e.g. right-click menus)",
                );
                Self::add_float_option(
                    &container,
                    &mut options,
                    "blur:popups_ignorealpha",
                    "Blur Popups Ignore Alpha",
                    "works like ignorealpha in layer rules. If pixel opacity is below set value, will not blur. [0.0 - 1.0]",
                );

                Self::add_section(
                    &container,
                    "Shadow",
                    "Configure shadow settings.",
                    first_section.clone(),
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "shadow:enabled",
                    "Shadow Enabled",
                    "enable drop shadows on windows",
                );
                Self::add_int_option(
                    &container,
                    &mut options,
                    "shadow:range",
                    "Shadow Range",
                    "Shadow range (“size”) in layout px",
                );
                Self::add_int_option(
                    &container,
                    &mut options,
                    "shadow:render_power",
                    "Shadow Render Power",
                    "in what power to render the falloff (more power, the faster the falloff) [1 - 4]",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "shadow:sharp",
                    "Shadow Sharp",
                    "if enabled, will make the shadows sharp, akin to an infinite render power",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "shadow:ignore_window",
                    "Shadow Ignore Window",
                    "if true, the shadow will not be rendered behind the window itself, only around it.",
                );
                Self::add_color_option(
                    &container,
                    &mut options,
                    "shadow:color",
                    "Shadow Color",
                    "shadow’s color. Alpha dictates shadow’s opacity.",
                );
                Self::add_color_option(
                    &container,
                    &mut options,
                    "shadow:color_inactive",
                    "Shadow Color Inactive",
                    "inactive shadow color. (if not set, will fall back to color)",
                );
                Self::add_string_option(
                    &container,
                    &mut options,
                    "offset",
                    "Shadow Offset",
                    "shadow’s rendering offset. [x, y]",
                );
                Self::add_float_option(
                    &container,
                    &mut options,
                    "shadow:scale",
                    "Shadow Scale",
                    "shadow’s scale. [0.0 - 1.0]",
                );
            }
            "animations" => {
                Self::add_section(
                    &container,
                    "Animation Settings",
                    "Configure animation behavior.",
                    first_section.clone(),
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "enabled",
                    "Enable Animations",
                    "Enables animations.",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "first_launch_animation",
                    "First Launch Animation",
                    "Enables the first launch animation.",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "workspace_wraparound",
                    "Workspace Wraparound",
                    "enable workspace wraparound, causing directional workspace animations to animate as if the first and last workspaces were adjacent",
                );
            }
            "input" => {
                Self::add_section(
                    &container,
                    "Input Settings",
                    "Configure input devices.",
                    first_section.clone(),
                );
                Self::add_section(
                    &container,
                    "Keyboard Settings",
                    "Configure keyboard behavior.",
                    first_section.clone(),
                );
                Self::add_string_option(
                    &container,
                    &mut options,
                    "kb_model",
                    "Keyboard Model",
                    "Appropriate XKB keymap parameter.",
                );
                Self::add_string_option(
                    &container,
                    &mut options,
                    "kb_layout",
                    "Keyboard Layout",
                    "Appropriate XKB keymap parameter",
                );
                Self::add_string_option(
                    &container,
                    &mut options,
                    "kb_variant",
                    "Keyboard Variant",
                    "Appropriate XKB keymap parameter",
                );
                Self::add_string_option(
                    &container,
                    &mut options,
                    "kb_options",
                    "Keyboard Options",
                    "Appropriate XKB keymap parameter",
                );
                Self::add_string_option(
                    &container,
                    &mut options,
                    "kb_rules",
                    "Keyboard Rules",
                    "Appropriate XKB keymap parameter",
                );
                Self::add_string_option(
                    &container,
                    &mut options,
                    "kb_file",
                    "Keyboard File",
                    "If you prefer, you can use a path to your custom .xkb file.",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "numlock_by_default",
                    "Numlock by Default",
                    "Engage numlock by default.",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "resolve_binds_by_sym",
                    "Resolve Binds by Symbol",
                    "Determines how keybinds act when multiple layouts are used. If false, keybinds will always act as if the first specified layout is active. If true, keybinds specified by symbols are activated when you type the respective symbol with the current layout.",
                );
                Self::add_int_option(
                    &container,
                    &mut options,
                    "repeat_rate",
                    "Repeat Rate",
                    "The repeat rate for held-down keys, in repeats per second.",
                );
                Self::add_int_option(
                    &container,
                    &mut options,
                    "repeat_delay",
                    "Repeat Delay",
                    "Delay before a held-down key is repeated, in milliseconds.",
                );

                Self::add_section(
                    &container,
                    "Mouse Settings",
                    "Configure mouse behavior.",
                    first_section.clone(),
                );
                Self::add_float_option(
                    &container,
                    &mut options,
                    "sensitivity",
                    "Sensitivity",
                    "Sets the mouse input sensitivity. Value is clamped to the range -1.0 to 1.0.",
                );
                Self::add_string_option(
                    &container,
                    &mut options,
                    "accel_profile",
                    "Acceleration Profile",
                    "Sets the cursor acceleration profile. Can be one of adaptive, flat. Can also be custom, see below. Leave empty to use libinput's default mode for your input device.",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "force_no_accel",
                    "Force No Acceleration",
                    "Force no cursor acceleration. This bypasses most of your pointer settings to get as raw of a signal as possible. Enabling this is not recommended due to potential cursor desynchronization.",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "left_handed",
                    "Left Handed",
                    "Switches RMB and LMB",
                );
                Self::add_string_option(
                    &container,
                    &mut options,
                    "scroll_method",
                    "Scroll Method",
                    "Sets the scroll method. Can be one of 2fg (2 fingers), edge, on_button_down, no_scroll.",
                );
                Self::add_int_option(
                    &container,
                    &mut options,
                    "scroll_button",
                    "Scroll Button",
                    "Sets the scroll button. Has to be an int, cannot be a string. Check wev if you have any doubts regarding the ID. 0 means default.",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "scroll_button_lock",
                    "Scroll Button Lock",
                    "If the scroll button lock is enabled, the button does not need to be held down. Pressing and releasing the button toggles the button lock, which logically holds the button down or releases it. While the button is logically held down, motion events are converted to scroll events.",
                );
                Self::add_float_option(
                    &container,
                    &mut options,
                    "scroll_factor",
                    "Scroll Factor",
                    "Multiplier added to scroll movement for external mice. Note that there is a separate setting for touchpad scroll_factor.",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "natural_scroll",
                    "Natural Scroll",
                    "Inverts scrolling direction. When enabled, scrolling moves content directly, rather than manipulating a scrollbar.",
                );
                Self::add_int_option(
                    &container,
                    &mut options,
                    "follow_mouse",
                    "Follow Mouse",
                    "Specify if and how cursor movement should affect window focus. 0 - Cursor movement will not change focus, 1 - Cursor movement will always change focus to the window under the cursor, 2 - Cursor focus will be detached from keyboard focus, 3 - Cursor focus will be completely separate from keyboard focus. [0/1/2/3]",
                );
                Self::add_float_option(
                    &container,
                    &mut options,
                    "follow_mouse_threshold",
                    "Follow Mouse Threshold",
                    "The smallest distance in logical pixels the mouse needs to travel for the window under it to get focused. Works only with follow_mouse = 1.",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "mouse_refocus",
                    "Mouse Refocus",
                    "If disabled, mouse focus won't switch to the hovered window unless the mouse crosses a window boundary when follow_mouse=1.",
                );
                Self::add_string_option(
                    &container,
                    &mut options,
                    "scroll_points",
                    "Scroll Points",
                    "Sets the scroll acceleration profile, when accel_profile is set to custom. Has to be in the form <step> <points>. Leave empty to have a flat scroll curve.",
                );

                Self::add_section(
                    &container,
                    "Focus Settings",
                    "Configure focus behavior.",
                    first_section.clone(),
                );
                Self::add_int_option(
                    &container,
                    &mut options,
                    "focus_on_close",
                    "Focus on Close",
                    "Controls the window focus behavior when a window is closed. 0 - focus will shift to the next window candidate, 1 - focus will shift to the window under the cursor. [0/1]",
                );
                Self::add_int_option(
                    &container,
                    &mut options,
                    "float_switch_override_focus",
                    "Float Switch Override Focus",
                    "If enabled, focus will change to the window under the cursor when changing from tiled-to-floating and vice versa. 0 - disabled, 1 - enabled, 2 - focus will also follow mouse on float-to-float switches. [0/1/2]",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "special_fallthrough",
                    "Special Fallthrough",
                    "if enabled, having only floating windows in the special workspace will not block focusing windows in the regular workspace.",
                );

                Self::add_section(
                    &container,
                    "Touchpad Settings",
                    "Configure touchpad behavior.",
                    first_section.clone(),
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "touchpad:disable_while_typing",
                    "Disable While Typing",
                    "Disables the touchpad while typing.",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "touchpad:natural_scroll",
                    "Natural Scroll",
                    "Inverts scrolling direction. When enabled, scrolling moves content directly, rather than manipulating a scrollbar.",
                );
                Self::add_float_option(
                    &container,
                    &mut options,
                    "touchpad:scroll_factor",
                    "Scroll Factor",
                    "Multiplier applied to the amount of scroll movement.",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "touchpad:middle_button_emulation",
                    "Middle Button Emulation",
                    "Sending LMB and RMB simultaneously will be interpreted as a middle click. This disables any touchpad area that would normally send a middle click based on location. libinput#middle-button-emulation",
                );
                Self::add_string_option(
                    &container,
                    &mut options,
                    "touchpad:tap_button_map",
                    "Tap Button Map",
                    "Sets the tap button mapping for touchpad button emulation. Can be one of lrm (default) or lmr (Left, Middle, Right Buttons). [lrm/lmr]",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "touchpad:clickfinger_behavior",
                    "Clickfinger Behavior",
                    "Button presses with 1, 2, or 3 fingers will be mapped to LMB, RMB, and MMB respectively. This disables interpretation of clicks based on location on the touchpad. libinput#clickfinger-behavior",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "touchpad:tap-to-click",
                    "Tap to Click",
                    "Tapping on the touchpad with 1, 2, or 3 fingers will send LMB, RMB, and MMB respectively.",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "touchpad:drag_lock",
                    "Drag Lock",
                    "drag_lock	When enabled, lifting the finger off while dragging will not drop the dragged item. 0 -> disabled, 1 -> enabled with timeout, 2 -> enabled sticky. libinput#tap-and-drag",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "touchpad:tap-and-drag",
                    "Tap and Drag",
                    "Sets the tap and drag mode for the touchpad",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "touchpad:flip_x",
                    "Flip X",
                    "inverts the horizontal movement of the touchpad",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "touchpad:flip_y",
                    "Flip Y",
                    "inverts the vertical movement of the touchpad",
                );
                Self::add_int_option(
                    &container,
                    &mut options,
                    "touchpad:drag_3fg",
                    "Drag 3FG",
                    "enables three finger drag, 0 -> disabled, 1 -> 3 fingers, 2 -> 4 fingers libinput#drag-3fg",
                );

                Self::add_section(
                    &container,
                    "Touchscreen Settings",
                    "Configure touchscreen behavior.",
                    first_section.clone(),
                );
                Self::add_int_option(
                    &container,
                    &mut options,
                    "touchdevice:transform",
                    "Transform",
                    "Transform the input from touchdevices. The possible transformations are the same as those of the monitors. -1 means it’s unset.",
                );
                Self::add_string_option(
                    &container,
                    &mut options,
                    "touchdevice:output",
                    "Output",
                    "The monitor to bind touch devices. The default is auto-detection. To stop auto-detection, use an empty string or the “[[Empty]]” value.",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "touchdevice:enabled",
                    "Enabled",
                    "Whether input is enabled for touch devices.",
                );

                Self::add_section(
                    &container,
                    "Tablet Settings",
                    "Configure tablet behavior.",
                    first_section.clone(),
                );
                Self::add_int_option(
                    &container,
                    &mut options,
                    "tablet:transform",
                    "Transform",
                    "transform the input from tablets. The possible transformations are the same as those of the monitors. -1 means it’s unset.",
                );
                Self::add_string_option(
                    &container,
                    &mut options,
                    "tablet:output",
                    "Output",
                    "the monitor to bind tablets. Can be current or a monitor name. Leave empty to map across all monitors.",
                );
                Self::add_string_option(
                    &container,
                    &mut options,
                    "tablet:region_position",
                    "Region Position",
                    "position of the mapped region in monitor layout relative to the top left corner of the bound monitor or all monitors. [x, y]",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "tablet:absolute_position",
                    "Absolute Position",
                    "whether to treat the region_position as an absolute position in monitor layout. Only applies when output is empty.",
                );
                Self::add_string_option(
                    &container,
                    &mut options,
                    "tablet:region_size",
                    "Region Size",
                    "size of the mapped region. When this variable is set, tablet input will be mapped to the region. [0, 0] or invalid size means unset. [x, y]",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "tablet:relative_input",
                    "Relative Input",
                    "whether the input should be relative",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "tablet:left_handed",
                    "Left Handed",
                    "if enabled, the tablet will be rotated 180 degrees",
                );
                Self::add_string_option(
                    &container,
                    &mut options,
                    "tablet:active_area_size",
                    "Active Area Size",
                    "size of tablet’s active area in mm [x, y]",
                );
                Self::add_string_option(
                    &container,
                    &mut options,
                    "tablet:active_area_position",
                    "Active Area Position",
                    "position of the active area in mm [x, y]",
                );

                Self::add_section(
                    &container,
                    "Miscellaneous Input Settings",
                    "Other input-related settings.",
                    first_section.clone(),
                );
                Self::add_int_option(
                    &container,
                    &mut options,
                    "off_window_axis_events",
                    "Off Window Axis Events",
                    "Handles axis events around a focused window. 0 - ignores axis events, 1 - sends out-of-bound coordinates, 2 - fakes pointer coordinates to the closest point inside the window, 3 - warps the cursor to the closest point inside the window [0/1/2/3]",
                );
                Self::add_int_option(
                    &container,
                    &mut options,
                    "emulate_discrete_scroll",
                    "Emulate Discrete Scroll",
                    "Emulates discrete scrolling from high resolution scrolling events. 0 - disables it, 1 - enables handling of non-standard events only, 2 - force enables all scroll wheel events to be handled [0/1/2]",
                );
            }
            "gestures" => {
                Self::add_section(
                    &container,
                    "Gesture Settings",
                    "Configure gesture behavior.",
                    first_section.clone(),
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "workspace_swipe",
                    "Workspace Swipe",
                    "enable workspace swipe gesture on touchpad",
                );
                Self::add_int_option(
                    &container,
                    &mut options,
                    "workspace_swipe_fingers",
                    "Workspace Swipe Fingers",
                    "how many fingers for the touchpad gesture",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "workspace_swipe_min_fingers",
                    "Workspace Swipe Min Fingers",
                    "if enabled, workspace_swipe_fingers is considered the minimum number of fingers to swipe",
                );
                Self::add_int_option(
                    &container,
                    &mut options,
                    "workspace_swipe_distance",
                    "Workspace Swipe Distance",
                    "in px, the distance of the touchpad gesture",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "workspace_swipe_touch",
                    "Workspace Swipe Touch",
                    "enable workspace swiping from the edge of a touchscreen",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "workspace_swipe_invert",
                    "Workspace Swipe Invert",
                    "invert the direction (touchpad only)",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "workspace_swipe_touch_invert",
                    "Workspace Swipe Touch Invert",
                    "invert the direction (touchscreen only)",
                );
                Self::add_int_option(
                    &container,
                    &mut options,
                    "workspace_swipe_min_speed_to_force",
                    "Workspace Swipe Min Speed to Force",
                    "minimum speed in px per timepoint to force the change ignoring cancel_ratio. Setting to 0 will disable this mechanic.",
                );
                Self::add_float_option(
                    &container,
                    &mut options,
                    "workspace_swipe_cancel_ratio",
                    "Workspace Swipe Cancel Ratio",
                    "how much the swipe has to proceed in order to commence it. (0.7 -> if > 0.7 * distance, switch, if less, revert) [0.0 - 1.0]",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "workspace_swipe_create_new",
                    "Workspace Swipe Create New",
                    "whether a swipe right on the last workspace should create a new one.",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "workspace_swipe_direction_lock",
                    "Workspace Swipe Direction Lock",
                    "if enabled, switching direction will be locked when you swipe past the direction_lock_threshold (touchpad only).",
                );
                Self::add_int_option(
                    &container,
                    &mut options,
                    "workspace_swipe_direction_lock_threshold",
                    "Workspace Swipe Direction Lock Threshold",
                    "in px, the distance to swipe before direction lock activates (touchpad only).",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "workspace_swipe_forever",
                    "Workspace Swipe Forever",
                    "if enabled, swiping will not clamp at the neighboring workspaces but continue to the further ones.",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "workspace_swipe_use_r",
                    "Workspace Swipe Use R",
                    "if enabled, swiping will use the r prefix instead of the m prefix for finding workspaces.",
                );
            }

            "group" => {
                Self::add_section(
                    &container,
                    "Group Settings",
                    "Configure group behavior.",
                    first_section.clone(),
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "auto_group",
                    "Auto Group",
                    "whether new windows will be automatically grouped into the focused unlocked group",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "insert_after_current",
                    "Insert After Current",
                    "whether new windows in a group spawn after current or at group tail",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "focus_removed_window",
                    "Focus Removed Window",
                    "whether Hyprland should focus on the window that has just been moved out of the group",
                );
                Self::add_int_option(
                    &container,
                    &mut options,
                    "drag_into_group",
                    "Drag Into Group",
                    "whether dragging a window into a unlocked group will merge them. 0 - disabled, 1 - enabled, 2 - only when dragging into the groupbar [0/1/2]",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "merge_groups_on_drag",
                    "Merge Groups on Drag",
                    "whether window groups can be dragged into other groups",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "merge_groups_on_groupbar",
                    "Merge Groups on Groupbar",
                    "whether one group will be merged with another when dragged into its groupbar",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "merge_floated_into_tiled_on_groupbar",
                    "Merge Floated Into Tiled on Groupbar",
                    "whether dragging a floating window into a tiled window groupbar will merge them",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "group_on_movetoworkspace",
                    "Group On MoveToWorkspace",
                    "whether using movetoworkspace[silent] will merge the window into the workspace’s solitary unlocked group",
                );
                Self::add_color_option(
                    &container,
                    &mut options,
                    "col.border_active",
                    "Active Border Color",
                    "active group border color",
                );
                Self::add_color_option(
                    &container,
                    &mut options,
                    "col.border_inactive",
                    "Inactive Border Color",
                    "inactive (out of focus) group border color",
                );
                Self::add_color_option(
                    &container,
                    &mut options,
                    "col.border_locked_active",
                    "Locked Active Border Color",
                    "active locked group border color",
                );
                Self::add_color_option(
                    &container,
                    &mut options,
                    "col.border_locked_inactive",
                    "Locked Inactive Border Color",
                    "inactive locked group border color",
                );
                Self::add_section(
                    &container,
                    "Groupbar Settings",
                    "Configure groupbar behavior.",
                    first_section.clone(),
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "groupbar:enabled",
                    "Enabled",
                    "enables groupbars",
                );
                Self::add_string_option(
                    &container,
                    &mut options,
                    "groupbar:font_family",
                    "Font Family",
                    "font used to display groupbar titles, use misc:font_family if not specified",
                );
                Self::add_int_option(
                    &container,
                    &mut options,
                    "groupbar:font_size",
                    "Font Size",
                    "font size of groupbar title",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "groupbar:gradients",
                    "Gradients",
                    "enables gradients",
                );
                Self::add_int_option(
                    &container,
                    &mut options,
                    "groupbar:height",
                    "Height",
                    "height of the groupbar",
                );
                Self::add_int_option(
                    &container,
                    &mut options,
                    "groupbar:indicator_gap",
                    "Indicator Gap",
                    "height of gap between groupbar indicator and title",
                );
                Self::add_int_option(
                    &container,
                    &mut options,
                    "groupbar:indicator_height",
                    "Indicator Height",
                    "height of the groupbar indicator",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "groupbar:stacked",
                    "Stacked",
                    "render the groupbar as a vertical stack",
                );
                Self::add_int_option(
                    &container,
                    &mut options,
                    "groupbar:priority",
                    "Priority",
                    "sets the decoration priority for groupbars",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "groupbar:render_titles",
                    "Render Titles",
                    "whether to render titles in the group bar decoration",
                );
                Self::add_int_option(
                    &container,
                    &mut options,
                    "groupbar:text_offset",
                    "Text Offset",
                    "adjust vertical position for titles",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "groupbar:scrolling",
                    "Scrolling",
                    "whether scrolling in the groupbar changes group active window",
                );
                Self::add_int_option(
                    &container,
                    &mut options,
                    "groupbar:rounding",
                    "Rounding",
                    "how much to round the indicator",
                );
                Self::add_int_option(
                    &container,
                    &mut options,
                    "groupbar:gradient_rounding",
                    "Gradient Rounding",
                    "how much to round the gradients",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "groupbar:round_only_edges",
                    "Round Only Edges",
                    "round only the indicator edges of the entire groupbar",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "groupbar:gradient_round_only_edges",
                    "Gradient Round Only Edges",
                    "round only the gradient edges of the entire groupbar",
                );
                Self::add_color_option(
                    &container,
                    &mut options,
                    "groupbar:text_color",
                    "Text Color",
                    "controls the group bar text color",
                );
                Self::add_color_option(
                    &container,
                    &mut options,
                    "groupbar:text_color_inactive",
                    "Inactive Text Color",
                    "color for inactive windows’ titles in the groupbar (if unset, defaults to text_color)",
                );
                Self::add_color_option(
                    &container,
                    &mut options,
                    "groupbar:text_color_locked_active",
                    "Locked Active Text Color",
                    "color for the active window’s title in a locked group (if unset, defaults to text_color)",
                );
                Self::add_color_option(
                    &container,
                    &mut options,
                    "groupbar:text_color_locked_inactive",
                    "Inactive Locked Text Color",
                    "color for inactive windows’ titles in locked groups (if unset, defaults to text_color_inactive)",
                );
                Self::add_color_option(
                    &container,
                    &mut options,
                    "groupbar:col.active",
                    "Active Color",
                    "active group border color",
                );
                Self::add_color_option(
                    &container,
                    &mut options,
                    "groupbar:col.inactive",
                    "Inactive Color",
                    "inactive (out of focus) group border color",
                );
                Self::add_color_option(
                    &container,
                    &mut options,
                    "groupbar:col.locked_active",
                    "Locked Active Color",
                    "active locked group border color",
                );
                Self::add_color_option(
                    &container,
                    &mut options,
                    "groupbar:col.locked_inactive",
                    "Locked Inactive Color",
                    "inactive locked group border color",
                );
                Self::add_int_option(
                    &container,
                    &mut options,
                    "groupbar:gaps_in",
                    "Gaps In",
                    "gap size between gradients",
                );
                Self::add_int_option(
                    &container,
                    &mut options,
                    "groupbar:gaps_out",
                    "Gaps Out",
                    "gap size between gradients and window",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "groupbar:keep_upper_gap",
                    "Keep Upper Gap",
                    "add or remove upper gap",
                );
            }
            "misc" => {
                Self::add_section(
                    &container,
                    "Miscellaneous Settings",
                    "Configure miscellaneous behavior.",
                    first_section.clone(),
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "disable_hyprland_logo",
                    "Disable Hyprland Logo",
                    "disables the random Hyprland logo / anime girl background. :(",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "disable_splash_rendering",
                    "Disable Splash Rendering",
                    "disables the Hyprland splash rendering. (requires a monitor reload to take effect)",
                );
                Self::add_color_option(
                    &container,
                    &mut options,
                    "col.splash",
                    "Splash Color",
                    "Changes the color of the splash text (requires a monitor reload to take effect).",
                );
                Self::add_string_option(
                    &container,
                    &mut options,
                    "font_family",
                    "Font Family",
                    "Set the global default font to render the text including debug fps/notification, config error messages and etc., selected from system fonts.",
                );
                Self::add_string_option(
                    &container,
                    &mut options,
                    "splash_font_family",
                    "Splash Font Family",
                    "Changes the font used to render the splash text, selected from system fonts (requires a monitor reload to take effect).",
                );
                Self::add_int_option(
                    &container,
                    &mut options,
                    "force_default_wallpaper",
                    "Force Default Wallpaper",
                    "Enforce any of the 3 default wallpapers. -1 - random, 0 or 1 - disables the anime background, 2 - enables anime background. [-1/0/1/2]",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "vfr",
                    "VFR",
                    "controls the VFR status of Hyprland. Heavily recommended to leave enabled to conserve resources.",
                );
                Self::add_int_option(
                    &container,
                    &mut options,
                    "vrr",
                    "VRR",
                    "Controls the VRR (Adaptive Sync) of your monitors. 0 - off, 1 - on, 2 - fullscreen only [0/1/2]",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "mouse_move_enables_dpms",
                    "Mouse Move Enables DPMS",
                    "If DPMS is set to off, wake up the monitors if the mouse moves.",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "key_press_enables_dpms",
                    "Key Press Enables DPMS",
                    "If DPMS is set to off, wake up the monitors if a key is pressed.",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "always_follow_on_dnd",
                    "Always Follow on DnD",
                    "Will make mouse focus follow the mouse when drag and dropping. Recommended to leave it enabled, especially for people using focus follows mouse at 0.",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "layers_hog_keyboard_focus",
                    "Layers Hog Keyboard Focus",
                    "If true, will make keyboard-interactive layers keep their focus on mouse move (e.g. wofi, bemenu)",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "animate_manual_resizes",
                    "Animate Manual Resizes",
                    "If true, will animate manual window resizes/moves",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "animate_mouse_windowdragging",
                    "Animate Mouse Window Dragging",
                    "If true, will animate windows being dragged by mouse, note that this can cause weird behavior on some curves",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "disable_autoreload",
                    "Disable Autoreload",
                    "If true, the config will not reload automatically on save, and instead needs to be reloaded with hyprctl reload. Might save on battery.",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "enable_swallow",
                    "Enable Swallow",
                    "Enable window swallowing",
                );
                Self::add_string_option(
                    &container,
                    &mut options,
                    "swallow_regex",
                    "Swallow Regex",
                    "The class regex to be used for windows that should be swallowed (usually, a terminal). To know more about the list of regex which can be used use this cheatsheet.",
                );
                Self::add_string_option(
                    &container,
                    &mut options,
                    "swallow_exception_regex",
                    "Swallow Exception Regex",
                    "The title regex to be used for windows that should not be swallowed by the windows specified in swallow_regex (e.g. wev). The regex is matched against the parent (e.g. Kitty) window's title on the assumption that it changes to whatever process it's running.",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "focus_on_activate",
                    "Focus on Activate",
                    "Whether Hyprland should focus an app that requests to be focused (an activate request)",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "mouse_move_focuses_monitor",
                    "Mouse Move Focuses Monitor",
                    "Whether mouse moving into a different monitor should focus it",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "allow_session_lock_restore",
                    "Allow Session Lock Restore",
                    "if true, will allow you to restart a lockscreen app in case it crashes (red screen of death)",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "session_lock_xray",
                    "Session Lock Xray",
                    "if true, keep rendering workspaces below your lockscreen",
                );
                Self::add_color_option(
                    &container,
                    &mut options,
                    "background_color",
                    "Background Color",
                    "change the background color. (requires enabled disable_hyprland_logo)",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "close_special_on_empty",
                    "Close Special on Empty",
                    "close the special workspace if the last window is removed",
                );
                Self::add_int_option(
                    &container,
                    &mut options,
                    "new_window_takes_over_fullscreen",
                    "New Window Takes Over Fullscreen",
                    "If there is a fullscreen or maximized window, decide whether a new tiled window opened should replace it, stay behind or disable the fullscreen/maximized state. 0 - behind, 1 - takes over, 2 - unfullscreen/unmaxize [0/1/2]",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "exit_window_retains_fullscreen",
                    "Exit Window Retains Fullscreen",
                    "if true, closing a fullscreen window makes the next focused window fullscreen",
                );
                Self::add_int_option(
                    &container,
                    &mut options,
                    "initial_workspace_tracking",
                    "Initial Workspace Tracking",
                    "If enabled, windows will open on the workspace they were invoked on. 0 - disabled, 1 - single-shot, 2 - persistent (all children too) [0/1/2]",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "middle_click_paste",
                    "Middle Click Paste",
                    "whether to enable middle-click-paste (aka primary selection)",
                );
                Self::add_int_option(
                    &container,
                    &mut options,
                    "render_unfocused_fps",
                    "Render Unfocused FPS",
                    "the maximum limit for renderunfocused windows' fps in the background (see also Window-Rules - renderunfocused)",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "disable_xdg_env_checks",
                    "Disable XDG Environment Checks",
                    "disable the warning if XDG environment is externally managed",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "disable_hyprland_qtutils_check",
                    "Disable Hyprland Qtutils Check",
                    "disable the warning if hyprland-qtutils is not installed",
                );
                Self::add_int_option(
                    &container,
                    &mut options,
                    "lockdead_screen_delay",
                    "Lock Dead Screen Delay",
                    "delay after which the “lockdead” screen will appear in case a lockscreen app fails to cover all the outputs (5 seconds max)",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "enable_anr_dialog",
                    "Enable ANR Dialog",
                    "whether to enable the ANR (app not responding) dialog when your apps hang",
                );
                Self::add_int_option(
                    &container,
                    &mut options,
                    "anr_missed_pings",
                    "ANR Missed Pings",
                    "number of missed pings before showing the ANR dialog",
                );
            }
            "binds" => {
                Self::add_section(
                    &container,
                    "Bind Settings",
                    "Configure keybinding behavior.",
                    first_section.clone(),
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "pass_mouse_when_bound",
                    "Pass Mouse When Bound",
                    "If disabled, will not pass the mouse events to apps / dragging windows around if a keybind has been triggered.",
                );
                Self::add_int_option(
                    &container,
                    &mut options,
                    "scroll_event_delay",
                    "Scroll Event Delay",
                    "In ms, how many ms to wait after a scroll event to allow passing another one for the binds.",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "workspace_back_and_forth",
                    "Workspace Back and Forth",
                    "If enabled, an attempt to switch to the currently focused workspace will instead switch to the previous workspace.",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "hide_special_on_workspace_change",
                    "Hide Special on Workspace Change",
                    "If enabled, changing the active workspace (including to itself) will hide the special workspace on the monitor where the newly active workspace resides.",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "allow_workspace_cycles",
                    "Allow Workspace Cycles",
                    "If enabled, workspaces don't forget their previous workspace, so cycles can be created.",
                );
                Self::add_int_option(
                    &container,
                    &mut options,
                    "workspace_center_on",
                    "Workspace Center On",
                    "Whether switching workspaces should center the cursor on the workspace (0) or on the last active window for that workspace (1). [0/1]",
                );
                Self::add_int_option(
                    &container,
                    &mut options,
                    "focus_preferred_method",
                    "Focus Preferred Method",
                    "Sets the preferred focus finding method when using focuswindow/movewindow/etc with a direction. 0 - history (recent have priority), 1 - length (longer shared edges have priority) [0/1]",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "ignore_group_lock",
                    "Ignore Group Lock",
                    "If enabled, dispatchers like moveintogroup, moveoutofgroup and movewindoworgroup will ignore lock per group.",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "movefocus_cycles_fullscreen",
                    "Movefocus Cycles Fullscreen",
                    "If enabled, when on a fullscreen window, movefocus will cycle fullscreen, if not, it will move the focus in a direction.",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "movefocus_cycles_groupfirst",
                    "Movefocus Cycles Groupfirst",
                    "If enabled, when in a grouped window, movefocus will cycle windows in the groups first, then at each ends of tabs, it’ll move on to other windows/groups.",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "disable_keybind_grabbing",
                    "Disable Keybind Grabbing",
                    "If enabled, apps that request keybinds to be disabled (e.g. VMs) will not be able to do so.",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "window_direction_monitor_fallback",
                    "Window Direction Monitor Fallback",
                    "If enabled, moving a window or focus over the edge of a monitor with a direction will move it to the next monitor in that direction.",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "allow_pin_fullscreen",
                    "Allow Pin Fullscreen",
                    "If enabled, Allow fullscreen to pinned windows, and restore their pinned status afterwards.",
                );
                Self::add_int_option(
                    &container,
                    &mut options,
                    "drag_threshold",
                    "Drag Threshold",
                    "Movement threshold in pixels for window dragging and c/g bind flags. 0 to disable and grab on mousedown.",
                );
            }
            "xwayland" => {
                Self::add_section(
                    &container,
                    "XWayland Settings",
                    "Configure XWayland behavior.",
                    first_section.clone(),
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "enabled",
                    "Enabled",
                    "Allow running applications using X11.",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "use_nearest_neighbor",
                    "Use Nearest Neighbor",
                    "Uses the nearest neighbor filtering for xwayland apps, making them pixelated rather than blurry.",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "force_zero_scaling",
                    "Force Zero Scaling",
                    "Forces a scale of 1 on xwayland windows on scaled displays.",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "create_abstract_socket",
                    "Create Abstract Socket",
                    "Create the abstract Unix domain socket for XWayland connections. (XWayland restart is required for changes to take effect; Linux only)",
                );
            }
            "opengl" => {
                Self::add_section(
                    &container,
                    "OpenGL Settings",
                    "Configure OpenGL behavior.",
                    first_section.clone(),
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "nvidia_anti_flicker",
                    "Nvidia Anti Flicker",
                    "Reduces flickering on nvidia at the cost of possible frame drops on lower-end GPUs.",
                );
            }
            "render" => {
                Self::add_section(
                    &container,
                    "Render Settings",
                    "Configure render behavior.",
                    first_section.clone(),
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "direct_scanout",
                    "Direct Scanout",
                    "Enables direct scanout. Direct scanout attempts to reduce lag when there is only one fullscreen application on a screen (e.g. game). It is also recommended to set this to false if the fullscreen application shows graphical glitches. 0 - off, 1 - on, 2 - auto (on with content type ‘game’)",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "expand_undersized_textures",
                    "Expand Undersized Textures",
                    "Whether to expand undersized textures along the edge, or rather stretch the entire texture.",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "xp_model",
                    "XP Model",
                    "Disables back buffer and bottom layer rendering.",
                );
                Self::add_int_option(
                    &container,
                    &mut options,
                    "ctm_animation",
                    "CTM Animation",
                    "Whether to enable a fade animation for CTM changes (hyprsunset). 2 means “auto” which disables them on Nvidia.",
                );
                Self::add_int_option(
                    &container,
                    &mut options,
                    "cm_fs_passthrough",
                    "CM FS Passthrough",
                    "Passthrough color settings for fullscreen apps when possible. 0 - off, 1 - always, 2 - hdr only",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "cm_enabled",
                    "CM Enabled",
                    "Whether the color management pipeline should be enabled or not (requires a restart of Hyprland to fully take effect)",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "send_content_type",
                    "Send Content Type",
                    "Report content type to allow monitor profile autoswitch (may result in a black screen during the switch).",
                );
                Self::add_int_option(
                    &container,
                    &mut options,
                    "cm_auto_hdr",
                    "CM Auto HDR",
                    "Auto-switch to HDR in fullscreen when needed. 0 - off, 1 - switch to cm, hdr, 2 - switch to cm, hdredid. [0/1/2]",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "new_render_scheduling",
                    "New Render Scheduling",
                    "Automatically uses triple buffering when needed, improves FPS on underpowered devices.",
                );
            }
            "cursor" => {
                Self::add_section(
                    &container,
                    "Cursor Settings",
                    "Configure cursor behavior.",
                    first_section.clone(),
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "sync_gsettings_theme",
                    "Sync GSettings Theme",
                    "Sync xcursor theme with gsettings.",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "no_hardware_cursors",
                    "No Hardware Cursors",
                    "Disables hardware cursors.",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "no_break_fs_vrr",
                    "No Break FS VRR",
                    "Disables scheduling new frames on cursor movement for fullscreen apps with VRR enabled to avoid framerate spikes.",
                );
                Self::add_int_option(
                    &container,
                    &mut options,
                    "min_refresh_rate",
                    "Min Refresh Rate",
                    "Minimum refresh rate for cursor movement when no_break_fs_vrr is active.",
                );
                Self::add_int_option(
                    &container,
                    &mut options,
                    "hotspot_padding",
                    "Hotspot Padding",
                    "The padding, in logical px, between screen edges and the cursor.",
                );
                Self::add_float_option(
                    &container,
                    &mut options,
                    "inactive_timeout",
                    "Inactive Timeout",
                    "In seconds, after how many seconds of cursor's inactivity to hide it.",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "no_warps",
                    "No Warps",
                    "If true, will not warp the cursor in many cases.",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "persistent_warps",
                    "Persistent Warps",
                    "When a window is refocused, the cursor returns to its last position relative to that window.",
                );
                Self::add_int_option(
                    &container,
                    &mut options,
                    "warp_on_change_workspace",
                    "Warp on Change Workspace",
                    "If true, move the cursor to the last focused window after changing the workspace. [0/1/2]",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "warp_on_toggle_special",
                    "Warp on Toggle Special",
                    "Move the cursor to the last focused window when toggling a special workspace. Options: 0 (Disabled), 1 (Enabled), 2 (Force - ignores cursor:no_warps option). [0/1/2]",
                );
                Self::add_string_option(
                    &container,
                    &mut options,
                    "default_monitor",
                    "Default Monitor",
                    "The name of a default monitor for the cursor to be set to on startup.",
                );
                Self::add_float_option(
                    &container,
                    &mut options,
                    "zoom_factor",
                    "Zoom Factor",
                    "The factor to zoom by around the cursor. Like a magnifying glass.",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "zoom_rigid",
                    "Zoom Rigid",
                    "Whether the zoom should follow the cursor rigidly or loosely.",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "enable_hyprcursor",
                    "Enable Hyprcursor",
                    "Whether to enable hyprcursor support.",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "hide_on_key_press",
                    "Hide on Key Press",
                    "Hides the cursor when you press any key until the mouse is moved.",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "hide_on_touch",
                    "Hide on Touch",
                    "Hides the cursor when the last input was a touch input until a mouse input is done.",
                );
                Self::add_int_option(
                    &container,
                    &mut options,
                    "use_cpu_buffer",
                    "Use CPU Buffer",
                    "Makes HW cursors use a CPU buffer. Required on Nvidia to have HW cursors. 0 - off, 1 - on, 2 - auto (nvidia only). [0/1/2]",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "warp_back_after_non_mouse_input",
                    "Warp Back After Non-Mouse Input",
                    "Warp the cursor back to where it was after using a non-mouse input to move it, and then returning back to mouse.",
                );
            }
            "ecosystem" => {
                Self::add_section(
                    &container,
                    "Ecosystem Settings",
                    "Configure ecosystem behavior.",
                    first_section.clone(),
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "no_update_news",
                    "No Update News",
                    "disable the popup that shows up when you update hyprland to a new version.",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "no_donation_nag",
                    "No Donation Nag",
                    "disable the popup that shows up twice a year encouraging to donate.",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "enforce_permissions",
                    "Enforce Permissions",
                    "whether to enable Hyprland's permission control.",
                );
            }
            "experimental" => {
                Self::add_section(
                    &container,
                    "Experimental Settings",
                    "Configure experimental behavior.",
                    first_section.clone(),
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "xx_color_management_v4",
                    "XX Color Management v4",
                    "enable color management protocol",
                );
            }
            "debug" => {
                Self::add_section(
                    &container,
                    "Debug Settings",
                    "Configure debug behavior.",
                    first_section.clone(),
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "overlay",
                    "Overlay",
                    "Print the debug performance overlay.",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "damage_blink",
                    "Damage Blink",
                    "(epilepsy warning!) Flash areas updated with damage tracking.",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "disable_logs",
                    "Disable Logs",
                    "Disable logging to a file.",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "disable_time",
                    "Disable Time",
                    "Disables time logging.",
                );
                Self::add_int_option(
                    &container,
                    &mut options,
                    "damage_tracking",
                    "Damage Tracking",
                    "Redraw only the needed bits of the display. Do not change. 0 - none, 1 - monitor, 2 - full (default) [0/1/2]",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "enable_stdout_logs",
                    "Enable Stdout Logs",
                    "Enables logging to stdout.",
                );
                Self::add_int_option(
                    &container,
                    &mut options,
                    "manual_crash",
                    "Manual Crash",
                    "Set to 1 and then back to 0 to crash Hyprland.",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "suppress_errors",
                    "Suppress Errors",
                    "If true, do not display config file parsing errors.",
                );
                Self::add_int_option(
                    &container,
                    &mut options,
                    "watchdog_timeout",
                    "Watchdog Timeout",
                    "Sets the timeout in seconds for watchdog to abort processing of a signal of the main thread. Set to 0 to disable.",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "disable_scale_checks",
                    "Disable Scale Checks",
                    "Disables verification of the scale factors. Will result in pixel alignment and rounding errors.",
                );
                Self::add_int_option(
                    &container,
                    &mut options,
                    "error_limit",
                    "Error Limit",
                    "Limits the number of displayed config file parsing errors.",
                );
                Self::add_int_option(
                    &container,
                    &mut options,
                    "error_position",
                    "Error Position",
                    "Sets the position of the error bar. 0 - top, 1 - bottom [0/1]",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "colored_stdout_logs",
                    "Colored Stdout Logs",
                    "Enables colors in the stdout logs.",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "pass",
                    "Pass",
                    "enables render pass debugging.",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "full_cm_proto",
                    "Full CM Proto",
                    "claims support for all cm proto features (requires restart).",
                );
            }
            "layouts" => {
                Self::add_section(
                    &container,
                    "Layout Settings",
                    "Configure layout behavior.",
                    first_section.clone(),
                );

                Self::add_section(
                    &container,
                    "Dwindle Layout",
                    "Configure Dwindle layout settings.",
                    first_section.clone(),
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "dwindle:pseudotile",
                    "Pseudotile",
                    "Enable pseudotiling. Pseudotiled windows retain their floating size when tiled.",
                );
                Self::add_int_option(
                    &container,
                    &mut options,
                    "dwindle:force_split",
                    "Force Split",
                    "0 -> split follows mouse, 1 -> always split to the left (new = left or top) 2 -> always split to the right (new = right or bottom)",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "dwindle:preserve_split",
                    "Preserve Split",
                    "If enabled, the split (side/top) will not change regardless of what happens to the container.",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "dwindle:smart_split",
                    "Smart Split",
                    "If enabled, allows a more precise control over the window split direction based on the cursor's position.",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "dwindle:smart_resizing",
                    "Smart Resizing",
                    "If enabled, resizing direction will be determined by the mouse's position on the window.",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "dwindle:permanent_direction_override",
                    "Permanent Direction Override",
                    "If enabled, makes the preselect direction persist until changed or disabled.",
                );
                Self::add_float_option(
                    &container,
                    &mut options,
                    "dwindle:special_scale_factor",
                    "Special Scale Factor",
                    "Specifies the scale factor of windows on the special workspace [0 - 1]",
                );
                Self::add_float_option(
                    &container,
                    &mut options,
                    "dwindle:split_width_multiplier",
                    "Split Width Multiplier",
                    "Specifies the auto-split width multiplier",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "dwindle:use_active_for_splits",
                    "Use Active for Splits",
                    "Whether to prefer the active window or the mouse position for splits",
                );
                Self::add_float_option(
                    &container,
                    &mut options,
                    "dwindle:default_split_ratio",
                    "Default Split Ratio",
                    "The default split ratio on window open. 1 means even 50/50 split. [0.1 - 1.9]",
                );
                Self::add_int_option(
                    &container,
                    &mut options,
                    "dwindle:split_bias",
                    "Split Bias",
                    "Specifies which window will receive the larger half of a split. [0/1/2]",
                );

                Self::add_section(
                    &container,
                    "Master Layout",
                    "Configure Master layout settings.",
                    first_section.clone(),
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "master:allow_small_split",
                    "Allow Small Split",
                    "Enable adding additional master windows in a horizontal split style",
                );
                Self::add_float_option(
                    &container,
                    &mut options,
                    "master:special_scale_factor",
                    "Special Scale Factor",
                    "The scale of the special workspace windows. [0.0 - 1.0]",
                );
                Self::add_float_option(
                    &container,
                    &mut options,
                    "master:mfact",
                    "Master Factor",
                    "The size as a percentage of the master window. [0.0 - 1.0]",
                );
                add_dropdown_option(
                    &container,
                    &mut options,
                    "master:new_status",
                    "New Window Status",
                    "Determines how new windows are added to the layout.",
                    &["master", "slave", "inherit"],
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "master:new_on_top",
                    "New on Top",
                    "Whether a newly open window should be on the top of the stack",
                );
                add_dropdown_option(
                    &container,
                    &mut options,
                    "master:new_on_active",
                    "New on Active",
                    "Place new window relative to the focused window",
                    &["before", "after", "none"],
                );
                add_dropdown_option(
                    &container,
                    &mut options,
                    "master:orientation",
                    "Orientation",
                    "Default placement of the master area",
                    &["left", "right", "top", "bottom", "center"],
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "master:inherit_fullscreen",
                    "Inherit Fullscreen",
                    "Inherit fullscreen status when cycling/swapping to another window",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "master:slave_count_for_center_master",
                    "Always Center Master",
                    "Keep the master window centered when using center orientation",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "master:smart_resizing",
                    "Smart Resizing",
                    "If enabled, resizing direction will be determined by the mouse's position on the window",
                );
                Self::add_bool_option(
                    &container,
                    &mut options,
                    "master:drop_at_cursor",
                    "Drop at Cursor",
                    "When enabled, dragging and dropping windows will put them at the cursor position",
                );
            }
            _ => {
                Self::add_section(
                    &container,
                    &format!("{category} Settings"),
                    &format!("Configure {category} behavior."),
                    first_section.clone(),
                );
            }
        }

        ConfigWidget {
            options,
            scrolled_window,
        }
    }

    fn add_section(
        container: &Box,
        title: &str,
        description: &str,
        first_section: Rc<RefCell<bool>>,
    ) {
        let section_box = Box::new(Orientation::Vertical, 5);
        section_box.set_margin_top(15);
        section_box.set_margin_bottom(10);

        let title_label = Label::new(Some(title));
        let desc_label = Label::new(Some(description));

        if *first_section.borrow() {
            title_label.set_halign(gtk::Align::Center);
            desc_label.set_halign(gtk::Align::Center);
            title_label.set_hexpand(true);
            desc_label.set_hexpand(true);
            *first_section.borrow_mut() = false;
        } else {
            title_label.set_halign(gtk::Align::Start);
            desc_label.set_halign(gtk::Align::Start);
        }

        title_label.set_markup(&format!("<b>{title}</b>"));
        section_box.append(&title_label);

        desc_label.set_opacity(0.7);
        section_box.append(&desc_label);

        let frame = Frame::new(None);
        frame.set_margin_top(10);
        section_box.append(&frame);

        container.append(&section_box);
    }

    fn add_int_option(
        container: &Box,
        options: &mut HashMap<String, Widget>,
        name: &str,
        label: &str,
        description: &str,
    ) {
        let hbox = Box::new(Orientation::Horizontal, 10);
        hbox.set_margin_start(10);
        hbox.set_margin_end(10);
        hbox.set_margin_top(5);
        hbox.set_margin_bottom(5);

        let label_box = Box::new(Orientation::Horizontal, 5);
        label_box.set_hexpand(true);

        let label_widget = Label::new(Some(label));
        label_widget.set_halign(gtk::Align::Start);

        let tooltip_button = Button::new();
        let question_mark_icon = Image::from_icon_name("dialog-question-symbolic");
        tooltip_button.set_child(Some(&question_mark_icon));
        tooltip_button.set_has_frame(false);

        let popover = Popover::new();
        let description_label = Label::new(Some(description));
        description_label.set_margin_top(5);
        description_label.set_margin_bottom(5);
        description_label.set_margin_start(5);
        description_label.set_margin_end(5);
        popover.set_child(Some(&description_label));
        popover.set_position(gtk::PositionType::Right);

        tooltip_button.connect_clicked(move |button| {
            popover.set_parent(button);
            popover.popup();
        });

        label_box.append(&label_widget);
        label_box.append(&tooltip_button);

        let (min, max, step) = get_option_limits(name, description);
        let spin_button = SpinButton::with_range(min, max, step);
        spin_button.set_digits(0);
        spin_button.set_halign(gtk::Align::End);
        spin_button.set_width_request(100);

        hbox.append(&label_box);
        hbox.append(&spin_button);

        container.append(&hbox);

        options.insert(name.to_string(), spin_button.upcast());
    }

    fn add_bool_option(
        container: &Box,
        options: &mut HashMap<String, Widget>,
        name: &str,
        label: &str,
        description: &str,
    ) {
        let hbox = Box::new(Orientation::Horizontal, 10);
        hbox.set_margin_start(10);
        hbox.set_margin_end(10);
        hbox.set_margin_top(5);
        hbox.set_margin_bottom(5);

        let label_box = Box::new(Orientation::Horizontal, 5);
        label_box.set_hexpand(true);

        let label_widget = Label::new(Some(label));
        label_widget.set_halign(gtk::Align::Start);

        let tooltip_button = Button::new();
        let question_mark_icon = Image::from_icon_name("dialog-question-symbolic");
        tooltip_button.set_child(Some(&question_mark_icon));
        tooltip_button.set_has_frame(false);

        let popover = Popover::new();
        let description_label = Label::new(Some(description));
        description_label.set_margin_top(5);
        description_label.set_margin_bottom(5);
        description_label.set_margin_start(5);
        description_label.set_margin_end(5);
        popover.set_child(Some(&description_label));
        popover.set_position(gtk::PositionType::Right);

        tooltip_button.connect_clicked(move |button| {
            popover.set_parent(button);
            popover.popup();
        });

        label_box.append(&label_widget);
        label_box.append(&tooltip_button);

        let switch = Switch::new();
        switch.set_halign(gtk::Align::End);
        switch.set_valign(gtk::Align::Center);

        hbox.append(&label_box);
        hbox.append(&switch);

        container.append(&hbox);

        options.insert(name.to_string(), switch.upcast());
    }

    fn add_float_option(
        container: &Box,
        options: &mut HashMap<String, Widget>,
        name: &str,
        label: &str,
        description: &str,
    ) {
        let hbox = Box::new(Orientation::Horizontal, 10);
        hbox.set_margin_start(10);
        hbox.set_margin_end(10);
        hbox.set_margin_top(5);
        hbox.set_margin_bottom(5);

        let label_box = Box::new(Orientation::Horizontal, 5);
        label_box.set_hexpand(true);

        let label_widget = Label::new(Some(label));
        label_widget.set_halign(gtk::Align::Start);

        let tooltip_button = Button::new();
        let question_mark_icon = Image::from_icon_name("dialog-question-symbolic");
        tooltip_button.set_child(Some(&question_mark_icon));
        tooltip_button.set_has_frame(false);

        let popover = Popover::new();
        let description_label = Label::new(Some(description));
        description_label.set_margin_top(5);
        description_label.set_margin_bottom(5);
        description_label.set_margin_start(5);
        description_label.set_margin_end(5);
        popover.set_child(Some(&description_label));
        popover.set_position(gtk::PositionType::Right);

        tooltip_button.connect_clicked(move |button| {
            popover.set_parent(button);
            popover.popup();
        });

        label_box.append(&label_widget);
        label_box.append(&tooltip_button);

        let (min, max, step) = get_option_limits(name, description);
        let spin_button = SpinButton::with_range(min, max, step);
        spin_button.set_digits(2);
        spin_button.set_halign(gtk::Align::End);
        spin_button.set_width_request(100);

        hbox.append(&label_box);
        hbox.append(&spin_button);

        container.append(&hbox);

        options.insert(name.to_string(), spin_button.upcast());
    }

    fn add_string_option(
        container: &Box,
        options: &mut HashMap<String, Widget>,
        name: &str,
        label: &str,
        description: &str,
    ) {
        let hbox = Box::new(Orientation::Horizontal, 10);
        hbox.set_margin_start(10);
        hbox.set_margin_end(10);
        hbox.set_margin_top(5);
        hbox.set_margin_bottom(5);

        let label_box = Box::new(Orientation::Horizontal, 5);
        label_box.set_hexpand(true);

        let label_widget = Label::new(Some(label));
        label_widget.set_halign(gtk::Align::Start);

        let tooltip_button = Button::new();
        let question_mark_icon = Image::from_icon_name("dialog-question-symbolic");
        tooltip_button.set_child(Some(&question_mark_icon));
        tooltip_button.set_has_frame(false);

        let popover = Popover::new();
        let description_label = Label::new(Some(description));
        description_label.set_margin_top(5);
        description_label.set_margin_bottom(5);
        description_label.set_margin_start(5);
        description_label.set_margin_end(5);
        popover.set_child(Some(&description_label));
        popover.set_position(gtk::PositionType::Right);

        tooltip_button.connect_clicked(move |button| {
            popover.set_parent(button);
            popover.popup();
        });

        label_box.append(&label_widget);
        label_box.append(&tooltip_button);

        let entry = Entry::new();
        entry.set_halign(gtk::Align::End);
        entry.set_width_request(100);

        hbox.append(&label_box);
        hbox.append(&entry);

        container.append(&hbox);

        options.insert(name.to_string(), entry.upcast());
    }

    fn add_color_option(
        container: &Box,
        options: &mut HashMap<String, Widget>,
        name: &str,
        label: &str,
        description: &str,
    ) {
        let hbox = Box::new(Orientation::Horizontal, 10);
        hbox.set_margin_start(10);
        hbox.set_margin_end(10);
        hbox.set_margin_top(5);
        hbox.set_margin_bottom(5);

        let label_box = Box::new(Orientation::Horizontal, 5);
        label_box.set_hexpand(true);

        let label_widget = Label::new(Some(label));
        label_widget.set_halign(gtk::Align::Start);

        let tooltip_button = Button::new();
        let question_mark_icon = Image::from_icon_name("dialog-question-symbolic");
        tooltip_button.set_child(Some(&question_mark_icon));
        tooltip_button.set_has_frame(false);

        let popover = Popover::new();
        let description_label = Label::new(Some(description));
        description_label.set_margin_top(5);
        description_label.set_margin_bottom(5);
        description_label.set_margin_start(5);
        description_label.set_margin_end(5);
        popover.set_child(Some(&description_label));
        popover.set_position(gtk::PositionType::Right);

        tooltip_button.connect_clicked(move |button| {
            popover.set_parent(button);
            popover.popup();
        });

        label_box.append(&label_widget);
        label_box.append(&tooltip_button);

        let color_dialog = ColorDialog::new();
        color_dialog.set_with_alpha(true);
        let color_button = ColorDialogButton::new(Some(color_dialog));
        color_button.set_halign(gtk::Align::End);

        let entry = Entry::new();
        entry.set_max_length(9);
        entry.set_width_chars(9);
        entry.set_placeholder_text(Some("#RRGGBBAA"));
        entry.set_halign(gtk::Align::End);

        {
            let rgba = color_button.rgba();
            let r = (rgba.red() * 255.0).round() as u8;
            let g = (rgba.green() * 255.0).round() as u8;
            let b = (rgba.blue() * 255.0).round() as u8;
            let a = (rgba.alpha() * 255.0).round() as u8;
            let hex = format!("#{r:02X}{g:02X}{b:02X}{a:02X}");
            entry.set_text(&hex);
        }

        hbox.append(&label_box);
        hbox.append(&color_button);
        hbox.append(&entry);
        container.append(&hbox);

        color_button.connect_rgba_notify(glib::clone!(
            #[weak]
            entry,
            move |btn| {
                let rgba = btn.rgba();
                let r = (rgba.red() * 255.0).round() as u8;
                let g = (rgba.green() * 255.0).round() as u8;
                let b = (rgba.blue() * 255.0).round() as u8;
                let a = (rgba.alpha() * 255.0).round() as u8;
                let hex = format!("#{r:02X}{g:02X}{b:02X}{a:02X}");
                entry.set_text(&hex);
            }
        ));

        entry.connect_changed(glib::clone!(
            #[weak]
            color_button,
            #[weak]
            entry,
            move |e| {
                let text = e.text().to_string();
                if let Ok(color) = gtk::gdk::RGBA::parse(&text) {
                    color_button.set_rgba(&color);
                    entry.set_css_classes(&[]);
                } else {
                    entry.set_css_classes(&["error"]);
                }
            }
        ));

        options.insert(name.to_string(), color_button.upcast());
    }

    fn load_config(
        &self,
        config: &HyprlandConfig,
        category: &str,
        changed_options: Rc<RefCell<HashMap<(String, String), String>>>,
    ) {
        for (name, widget) in &self.options {
            let value = self.extract_value(config, category, name);
            if let Some(spin_button) = widget.downcast_ref::<SpinButton>() {
                let float_value = value.parse::<f64>().unwrap_or(0.0);
                spin_button.set_value(float_value);
                let category = category.to_string();
                let name = name.to_string();
                let changed_options = changed_options.clone();
                spin_button.connect_value_changed(move |sb| {
                    let mut changes = changed_options.borrow_mut();
                    let new_value = sb.value().to_string();
                    changes.insert((category.clone(), name.clone()), new_value);
                });
            } else if let Some(entry) = widget.downcast_ref::<Entry>() {
                entry.set_text(&value);
                let category = category.to_string();
                let name = name.to_string();
                let changed_options = changed_options.clone();
                entry.connect_changed(move |entry| {
                    let mut changes = changed_options.borrow_mut();
                    let new_value = entry.text().to_string();
                    changes.insert((category.clone(), name.clone()), new_value);
                });
            } else if let Some(switch) = widget.downcast_ref::<Switch>() {
                switch.set_active(value == "true");
                let category = category.to_string();
                let name = name.to_string();
                let changed_options = changed_options.clone();
                switch.connect_active_notify(move |sw| {
                    let mut changes = changed_options.borrow_mut();
                    let new_value = sw.is_active().to_string();
                    changes.insert((category.clone(), name.clone()), new_value);
                });
            } else if let Some(color_button) = widget.downcast_ref::<ColorDialogButton>() {
                if let Some((red, green, blue, alpha)) = config.parse_color(&value) {
                    color_button.set_rgba(&gdk::RGBA::new(red, green, blue, alpha));
                }
                let category = category.to_string();
                let name = name.to_string();
                let changed_options = changed_options.clone();
                color_button.connect_rgba_notify(move |cb| {
                    let mut changes = changed_options.borrow_mut();
                    let new_color = cb.rgba();
                    let new_value = format!(
                        "rgba({:02X}{:02X}{:02X}{:02X})",
                        (new_color.red() * 255.0) as u8,
                        (new_color.green() * 255.0) as u8,
                        (new_color.blue() * 255.0) as u8,
                        (new_color.alpha() * 255.0) as u8
                    );
                    changes.insert((category.clone(), name.clone()), new_value);
                });
            } else if let Some(dropdown) = widget.downcast_ref::<gtk::DropDown>() {
                let model = dropdown.model().unwrap();
                for i in 0..model.n_items() {
                    if let Some(item) = model.item(i) {
                        if let Some(string_object) = item.downcast_ref::<gtk::StringObject>() {
                            if string_object.string() == value {
                                dropdown.set_selected(i);
                                break;
                            }
                        }
                    }
                }
                let category = category.to_string();
                let name = name.to_string();
                let changed_options = changed_options.clone();
                dropdown.connect_selected_notify(move |dd| {
                    let mut changes = changed_options.borrow_mut();
                    if let Some(selected) = dd.selected_item() {
                        if let Some(string_object) = selected.downcast_ref::<gtk::StringObject>() {
                            let new_value = string_object.string().to_string();
                            changes.insert((category.clone(), name.clone()), new_value);
                        }
                    }
                });
            }
        }
    }

    fn extract_value(&self, config: &HyprlandConfig, category: &str, name: &str) -> String {
        let config_str = self.transform_config(config.to_string());
        for line in config_str.lines() {
            if line.trim().starts_with(&format!("{category}:{name} = ")) {
                return line
                    .split('=')
                    .nth(1)
                    .map(|s| s.trim().to_string())
                    .unwrap_or_default();
            }
        }
        String::new()
    }

    // transform from general{snap{enabled = true}} to general:snap:enabled = true
    fn transform_config(&self, input: String) -> String {
        let mut result = Vec::new();
        let mut path = VecDeque::new();

        for line in input.lines() {
            let line = line.trim();
            if line.ends_with('{') {
                // start of the block
                let key = line.trim_end_matches('{').trim();
                path.push_back(key.to_string());
            } else if line == "}" {
                // end of the block
                path.pop_back();
            } else if line.contains('=') {
                let mut parts = line.splitn(2, '=');
                let key = parts.next().unwrap().trim();
                let value = parts.next().unwrap().trim();
                let prefix = path.iter().cloned().collect::<Vec<_>>().join(":");
                let full_key = format!("{prefix}:{key}");
                result.push(format!("{full_key} = {value}"));
            }
        }

        result.join("\n")
    }
}
