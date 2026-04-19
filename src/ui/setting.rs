use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender,
    Controller, SimpleComponent, adw, gtk,
};
use relm4::adw::prelude::*;
use relm4::gtk::prelude::*;

// ============================================================
//  Settings Dialog Component
// ============================================================

/// 设置对话框的内部状态
pub struct Settings {
    /// 主题下拉列表的数据模型
    theme_list: gtk::StringList,
    /// 当前选中的主题索引 (0=跟随系统, 1=浅色, 2=深色)
    theme_index: u32,
    /// 是否启用动态背景
    dynamic_background: bool,
    /// 启动时是否检查更新
    check_updates_on_start: bool,
    /// 关闭窗口时是否最小化到托盘
    minimize_to_tray: bool,
}

/// 设置对话框接收的消息
#[derive(Debug)]
pub enum SettingsInput {
    /// 主题切换
    ThemeChanged(u32),
    /// 动态背景开关
    DynamicBackgroundToggled(bool),
    /// 启动检查更新开关
    CheckUpdatesToggled(bool),
    /// 最小化到托盘开关
    MinimizeToTrayToggled(bool),
    /// 重置所有设置
    ResetSettings,
    /// 重新加载设置 (打开对话框时触发)
    ReloadAll,
}

/// 设置对话框向父组件发送的信号
#[derive(Debug)]
pub enum SettingsOutput {
    /// 主题被更改
    ThemeChanged(u32),
    /// 动态背景设置被更改
    DynamicBackgroundChanged(bool),
}

#[relm4::component(pub)]
impl SimpleComponent for Settings {
    type Init = ();
    type Input = SettingsInput;
    type Output = SettingsOutput;

    view! {
        #[name(dialog)]
        adw::PreferencesDialog {
            set_title: "Settings",

            // ============ 第一页：通用设置 ============
            add = &adw::PreferencesPage {
                set_title: "General",
                set_icon_name: Some("preferences-system-symbolic"),

                // ---------- 外观设置组 ----------
                adw::PreferencesGroup {
                    set_title: "Appearance",
                    set_description: Some("Customize the look and feel"),

                    // 主题选择
                    #[name(theme_row)]
                    adw::ComboRow {
                        set_title: "Theme",
                        set_subtitle: "Select application color scheme",

                        add_prefix = &gtk::Image {
                            set_icon_name: Some("palette-symbolic"),
                        },

                        set_model: Some(&model.theme_list),

                        #[watch]
                        set_selected: model.theme_index,

                        connect_selected_notify[sender] => move |row| {
                            sender.input_sender().emit(SettingsInput::ThemeChanged(row.selected()));
                        },
                    },

                    // 动态背景开关
                    adw::SwitchRow {
                        set_title: "Dynamic Background",
                        set_subtitle: "Change background based on content",

                        add_prefix = &gtk::Image {
                            set_icon_name: Some("image-alt-symbolic"),
                        },

                        #[watch]
                        set_active: model.dynamic_background,

                        connect_active_notify[sender] => move |switch| {
                            sender.input_sender().emit(
                                SettingsInput::DynamicBackgroundToggled(switch.is_active())
                            );
                        },
                    },
                },

                // ---------- 行为设置组 ----------
                adw::PreferencesGroup {
                    set_title: "Behavior",
                    set_description: Some("Configure application behavior"),

                    // 检查更新开关
                    adw::SwitchRow {
                        set_title: "Check Updates on Start",
                        set_subtitle: "Automatically check for updates when the app launches",

                        add_prefix = &gtk::Image {
                            set_icon_name: Some("software-update-symbolic"),
                        },

                        #[watch]
                        set_active: model.check_updates_on_start,

                        connect_active_notify[sender] => move |switch| {
                            sender.input_sender().emit(
                                SettingsInput::CheckUpdatesToggled(switch.is_active())
                            );
                        },
                    },

                    // 最小化到托盘开关
                    adw::SwitchRow {
                        set_title: "Minimize to Tray",
                        set_subtitle: "Keep running in the background when closed",

                        add_prefix = &gtk::Image {
                            set_icon_name: Some("system-tray-symbolic"),
                        },

                        #[watch]
                        set_active: model.minimize_to_tray,

                        connect_active_notify[sender] => move |switch| {
                            sender.input_sender().emit(
                                SettingsInput::MinimizeToTrayToggled(switch.is_active())
                            );
                        },
                    },
                },

                // ---------- 关于 & 重置 ----------
                adw::PreferencesGroup {
                    set_title: "About",

                    // 版本信息
                    adw::ActionRow {
                        set_title: "Version",
                        set_subtitle: env!("CARGO_PKG_VERSION"),

                        add_prefix = &gtk::Image {
                            set_icon_name: Some("help-about-symbolic"),
                        },
                    },

                    // 重置按钮
                    adw::ButtonRow {
                        set_title: "Reset All Settings",
                        set_start_icon_name: Some("view-refresh-symbolic"),
                        add_css_class: "destructive-action",

                        connect_activated[sender] => move |_| {
                            sender.input_sender().emit(SettingsInput::ResetSettings);
                        },
                    },
                },
            },

            // 每次打开对话框时重新加载设置
            connect_map[sender] => move |_| {
                sender.input_sender().emit(SettingsInput::ReloadAll);
            },
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        // 创建主题选项列表
        let theme_list = gtk::StringList::new(&["Follow System", "Light", "Dark"]);

        // ---- 加载初始设置 ----
        // 提示：实际项目中替换为 gio::Settings 持久化读取，例如：
        //   let settings = gio::Settings::new("com.example.myapp");
        //   let theme_str = settings.string("theme");
        //   let theme_index = match theme_str.as_str() {
        //       "follow-system" => 0,
        //       "light"         => 1,
        //       "dark"          => 2,
        //       _               => 0,
        //   };
        let model = Self {
            theme_list,
            theme_index: 0,
            dynamic_background: true,
            check_updates_on_start: true,
            minimize_to_tray: false,
        };

        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>) {
        match message {
            SettingsInput::ThemeChanged(index) => {
                self.theme_index = index;
                // 持久化：settings.set_string("theme", theme_name).ok();
                sender.output(SettingsOutput::ThemeChanged(index)).ok();
            }
            SettingsInput::DynamicBackgroundToggled(active) => {
                self.dynamic_background = active;
                // 持久化：settings.set_boolean("dynamic-background", active).ok();
                sender.output(SettingsOutput::DynamicBackgroundChanged(active)).ok();
            }
            SettingsInput::CheckUpdatesToggled(active) => {
                self.check_updates_on_start = active;
            }
            SettingsInput::MinimizeToTrayToggled(active) => {
                self.minimize_to_tray = active;
            }
            SettingsInput::ResetSettings => {
                // 恢复默认值
                self.theme_index = 0;
                self.dynamic_background = true;
                self.check_updates_on_start = true;
                self.minimize_to_tray = false;
                // 持久化：settings.reset("theme") 等
                // 通知父组件主题已重置
                sender.output(SettingsOutput::ThemeChanged(0)).ok();
                sender.output(SettingsOutput::DynamicBackgroundChanged(true)).ok();
            }
            SettingsInput::ReloadAll => {
                // 从 gio::Settings 重新加载所有字段
            }
        }
    }
}

// ============================================================
//  Main Application Window
// ============================================================

// struct App {
//     settings_dialog: Controller<SettingsDialog>,
//     main_window: adw::ApplicationWindow,
// }

// #[derive(Debug)]
// enum AppInput {
//     OpenSettings,
//     ThemeChanged(u32),
//     DynamicBackgroundChanged(bool),
// }

// #[relm4::component]
// impl SimpleComponent for App {
//     type Init = ();
//     type Input = AppInput;
//     type Output = ();
//     type Root = adw::ApplicationWindow;

//     view! {
//         adw::ApplicationWindow {
//             set_title: "Settings Dialog Demo".to_string(),
//             set_default_width: 500,
//             set_default_height: 400,

//             #[wrap(Some)]
//             set_content = &gtk::Box {
//                 set_orientation: gtk::Orientation::Vertical,
//                 set_valign: gtk::Align::Center,
//                 set_halign: gtk::Align::Center,
//                 set_spacing: 16,

//                 gtk::Image {
//                     set_icon_name: Some("preferences-system-symbolic"),
//                     set_pixel_size: 80,
//                 },

//                 gtk::Label {
//                     set_text: "Settings Dialog Example",
//                     add_css_class: "title-1",
//                 },

//                 gtk::Label {
//                     set_text: "Click the button below to open a native Adwaita settings dialog.",
//                     add_css_class: "body",
//                     set_margin_start: 48,
//                     set_margin_end: 48,
//                 },

//                 gtk::Button {
//                     set_label: "Open Settings",
//                     add_css_class: "pill",
//                     add_css_class: "suggested-action",
//                     set_margin_top: 24,

//                     connect_clicked => AppInput::OpenSettings,
//                 },
//             },
//         }
//     }

//     fn init(
//         _init: Self::Init,
//         root: Self::Root,
//         sender: ComponentSender<Self>,
//     ) -> ComponentParts<Self> {
//         // 创建设置对话框子组件，并将其输出转发为主窗口的输入
//         let settings_dialog = SettingsDialog::builder()
//             .launch(())
//             .forward(sender.input_sender(), |output| match output {
//                 SettingsOutput::ThemeChanged(i) => AppInput::ThemeChanged(i),
//                 SettingsOutput::DynamicBackgroundChanged(b) => AppInput::DynamicBackgroundChanged(b),
//             });

//         // 保存主窗口引用，用于作为对话框的父窗口
//         let model = App {
//             settings_dialog,
//             main_window: root.clone(),
//         };

//         let widgets = view_output!();
//         ComponentParts { model, widgets }
//     }

//     fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>) {
//         match message {
//             AppInput::OpenSettings => {
//                 // 以当前窗口为父窗口弹出设置对话框
//                 self.settings_dialog.widget().present(Some(&self.main_window));
//             }
//             AppInput::ThemeChanged(index) => {
//                 // 实际切换应用主题
//                 let app = relm4::main_adw_application();
//                 let style_manager = app.style_manager();
//                 let scheme = match index {
//                     0 => adw::ColorScheme::Default,
//                     1 => adw::ColorScheme::ForceLight,
//                     2 => adw::ColorScheme::ForceDark,
//                     _ => adw::ColorScheme::Default,
//                 };
//                 style_manager.set_color_scheme(scheme);
//             }
//             AppInput::DynamicBackgroundChanged(active) => {
//                 println!("Dynamic background setting changed: {active}");
//             }
//         }
//     }
// }
