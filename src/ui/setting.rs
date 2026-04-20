use relm4::adw::prelude::*;
use relm4::gtk::gio;
use relm4::{ ComponentParts, ComponentSender, SimpleComponent,
    adw, gtk,
};
use strum::Display;

use crate::APPLICATION_ID;

mod keys {
    pub const THEME: &str = "theme";
    pub const DYNAMIC_BACKGROUND: &str = "dynamic-background";
    pub const CHECK_UPDATES_ON_START: &str = "check-updates-on-start";
    pub const MINIMIZE_TO_TRAY: &str = "minimize-to-tray";
    pub const COOKIE: &str = "cookie";
}

#[derive(Debug, Clone, PartialEq, Display)]
pub enum Theme {
    #[strum(serialize = "follow-system")]
    FollowSystem,
    #[strum(serialize = "light")]
    Light,
    #[strum(serialize = "dark")]
    Dark,
}

impl Theme {
    const ALL: [Self; 3] = [Self::FollowSystem, Self::Light, Self::Dark];
    const LABELS: [&str; 3] = ["跟随系统", "浅色", "深色"];

    fn index(&self) -> u32 {
        match self {
            Self::FollowSystem => 0,
            Self::Light => 1,
            Self::Dark => 2,
        }
    }

    fn from_index(index: u32) -> Self {
        Self::ALL.get(index as usize).cloned().unwrap_or(Self::FollowSystem)
    }

    fn from_str_lossy(s: &str) -> Self {
        match s {
            "follow-system" => Self::FollowSystem,
            "light" => Self::Light,
            "dark" => Self::Dark,
            _ => Self::FollowSystem,
        }
    }
}

pub struct Settings {
    settings: gio::Settings,
    theme_list: gtk::StringList,
    theme: Theme,
    dynamic_background: bool,
    check_updates_on_start: bool,
    minimize_to_tray: bool,
    cookie: String,
}

#[derive(Debug)]
pub enum SettingsInput {
    ThemeChanged(u32),
    DynamicBackgroundToggled(bool),
    CheckUpdatesToggled(bool),
    MinimizeToTrayToggled(bool),
    UserCookieChanged(String),
    SaveCookie(String),
    ResetSettings,
    ReloadAll,
}

#[derive(Debug)]
pub enum SettingsOutput {
    ThemeChanged(Theme),
    DynamicBackgroundChanged(bool),
    UserCookieChanged(String),
    SaveCookie,
}

#[relm4::component(pub)]
impl SimpleComponent for Settings {
    type Init = ();
    type Input = SettingsInput;
    type Output = SettingsOutput;

    view! {
        #[name(dialog)]
        adw::PreferencesDialog {
            set_title: "设置",

            add = &adw::PreferencesPage {
                set_title: "通用",
                set_icon_name: Some("preferences-system-symbolic"),

                adw::PreferencesGroup {
                    set_title: "外观",
                    set_description: Some("自定义外观和体验"),

                    #[name(theme_row)]
                    adw::ComboRow {
                        set_title: "主题",
                        set_subtitle: "选择应用配色方案",

                        add_prefix = &gtk::Image {
                            set_icon_name: Some("dark-mode"),
                        },

                        set_model: Some(&model.theme_list),

                        #[watch]
                        set_selected: model.theme.index(),

                        connect_selected_notify[sender] => move |row| {
                            sender.input_sender().emit(SettingsInput::ThemeChanged(row.selected()));
                        },
                    },

                    adw::SwitchRow {
                        set_title: "动态背景",
                        set_subtitle: "根据内容更改背景",

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

                adw::PreferencesGroup {
                    set_title: "账户",
                    set_description: Some("Cookies"),

                    #[name(cookie_entry)]
                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 6,

                        gtk::Label {
                            set_label: "用户Cookie",
                            set_halign: gtk::Align::Start,
                        },

                        gtk::Entry {
                            set_text: &model.cookie,

                            connect_activate[sender] => move |entry| {
                                sender.input_sender().emit(
                                    SettingsInput::SaveCookie(entry.text().to_string())
                                );
                            }
                        }
                    }
                },

                adw::PreferencesGroup {
                    set_title: "行为",
                    set_description: Some("配置应用行为"),

                    adw::SwitchRow {
                        set_title: "启动时检查更新",
                        set_subtitle: "应用启动时自动检查更新",

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

                    adw::SwitchRow {
                        set_title: "最小化到托盘",
                        set_subtitle: "关闭时在后台保持运行",

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

                adw::PreferencesGroup {
                    set_title: "关于",

                    adw::ActionRow {
                        set_title: "版本",
                        set_subtitle: env!("CARGO_PKG_VERSION"),

                        add_prefix = &gtk::Image {
                            set_icon_name: Some("help-about-symbolic"),
                        },
                    },

                    adw::ButtonRow {
                        set_title: "重置所有设置",
                        set_start_icon_name: Some("view-refresh-symbolic"),
                        add_css_class: "destructive-action",

                        connect_activated[sender] => move |_| {
                            sender.input_sender().emit(SettingsInput::ResetSettings);
                        },
                    },
                },
            },

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
        let theme_list = gtk::StringList::new(&Theme::LABELS);

        let settings = gio::Settings::new(APPLICATION_ID);
        let theme = Theme::from_str_lossy(&settings.string(keys::THEME));
        let cookie = settings.string(keys::COOKIE).to_string();
        let check_updates_on_start = settings.boolean(keys::CHECK_UPDATES_ON_START);
        let minimize_to_tray = settings.boolean(keys::MINIMIZE_TO_TRAY);
        let dynamic_background = settings.boolean(keys::DYNAMIC_BACKGROUND);

        let model = Self {
            settings,
            theme_list,
            theme,
            dynamic_background,
            check_updates_on_start,
            minimize_to_tray,
            cookie,
        };

        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

        fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>) {
        match message {
            SettingsInput::ThemeChanged(index) => {
                self.theme = Theme::from_index(index);
                self.settings.set_string(keys::THEME, self.theme.to_string().as_str()).ok();
                sender.output(SettingsOutput::ThemeChanged(self.theme.clone())).ok();
            }
            SettingsInput::DynamicBackgroundToggled(active) => {
                self.dynamic_background = active;
                self.settings.set_boolean(keys::DYNAMIC_BACKGROUND, active).ok();
                sender.output(SettingsOutput::DynamicBackgroundChanged(active)).ok();
            }
            SettingsInput::CheckUpdatesToggled(active) => {
                self.check_updates_on_start = active;
                self.settings.set_boolean(keys::CHECK_UPDATES_ON_START, active).ok();
            }
            SettingsInput::MinimizeToTrayToggled(active) => {
                self.minimize_to_tray = active;
                self.settings.set_boolean(keys::MINIMIZE_TO_TRAY, active).ok();
            }
            
            // 【关键修改点】
            SettingsInput::UserCookieChanged(_text) => {
                // 留空，不更新 self.cookie
            }
            
            SettingsInput::SaveCookie(text) => {
                self.cookie = text.clone();
                self.settings.set_string(keys::COOKIE, &text).ok();
                sender.output(SettingsOutput::SaveCookie).ok();
            }
            SettingsInput::ResetSettings => {
                self.theme = Theme::FollowSystem;
                self.dynamic_background = true;
                self.check_updates_on_start = true;
                self.minimize_to_tray = false;
                self.cookie = String::new();
                sender.output(SettingsOutput::ThemeChanged(Theme::FollowSystem)).ok();
                sender.output(SettingsOutput::DynamicBackgroundChanged(true)).ok();
                sender.output(SettingsOutput::UserCookieChanged(String::new())).ok();
            }
            SettingsInput::ReloadAll => {
                self.theme = Theme::from_str_lossy(&self.settings.string(keys::THEME));
                self.dynamic_background = self.settings.boolean(keys::DYNAMIC_BACKGROUND);
                self.check_updates_on_start = self.settings.boolean(keys::CHECK_UPDATES_ON_START);
                self.minimize_to_tray = self.settings.boolean(keys::MINIMIZE_TO_TRAY);
                self.cookie = self.settings.string(keys::COOKIE).to_string();
            }
        }
    }

}
