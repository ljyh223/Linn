use relm4::adw::prelude::*;
use relm4::gtk::gio;
use relm4::{ComponentParts, ComponentSender, SimpleComponent, adw, gtk};

use crate::APPLICATION_ID;

mod keys {
    pub const RESTORE_ON_START: &str = "restore-on-start";
    pub const AUTO_PLAY_ON_RESTORE: &str = "auto-play-on-restore";
    pub const COOKIE: &str = "cookie";
}

pub struct Settings {
    settings: gio::Settings,
    restore_on_start: bool,
    auto_play_on_restore: bool,
    cookie: String,
}

#[derive(Debug)]
pub enum SettingsInput {
    RestoreOnStartToggled(bool),
    AutoPlayOnRestoreToggled(bool),
    UserCookieChanged(String),
    SaveCookie(String),
    ResetSettings,
    ReloadAll,
}

#[derive(Debug)]
pub enum SettingsOutput {
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
                        set_title: "启动时恢复上次播放",
                        set_subtitle: "启动后自动恢复上次播放的歌曲",

                        add_prefix = &gtk::Image {
                            set_icon_name: Some("media-playlist-repeat-symbolic"),
                        },

                        #[watch]
                        set_active: model.restore_on_start,

                        connect_active_notify[sender] => move |switch| {
                            sender.input_sender().emit(
                                SettingsInput::RestoreOnStartToggled(switch.is_active())
                            );
                        },
                    },

                    adw::SwitchRow {
                        set_title: "恢复后自动播放",
                        set_subtitle: "恢复上次播放后立即开始播放，关闭则停留在暂停状态",

                        add_prefix = &gtk::Image {
                            set_icon_name: Some("media-playback-start-symbolic"),
                        },

                        #[watch]
                        set_active: model.auto_play_on_restore,

                        connect_active_notify[sender] => move |switch| {
                            sender.input_sender().emit(
                                SettingsInput::AutoPlayOnRestoreToggled(switch.is_active())
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
        let settings = gio::Settings::new(APPLICATION_ID);
        let cookie = settings.string(keys::COOKIE).to_string();
        let restore_on_start = settings.boolean(keys::RESTORE_ON_START);
        let auto_play_on_restore = settings.boolean(keys::AUTO_PLAY_ON_RESTORE);

        let model = Self {
            settings,
            restore_on_start,
            auto_play_on_restore,
            cookie,
        };

        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>) {
        match message {
            SettingsInput::RestoreOnStartToggled(active) => {
                self.restore_on_start = active;
                self.settings
                    .set_boolean(keys::RESTORE_ON_START, active)
                    .ok();
            }
            SettingsInput::AutoPlayOnRestoreToggled(active) => {
                self.auto_play_on_restore = active;
                self.settings
                    .set_boolean(keys::AUTO_PLAY_ON_RESTORE, active)
                    .ok();
            }

            SettingsInput::UserCookieChanged(_text) => {}

            SettingsInput::SaveCookie(text) => {
                self.cookie = text.clone();
                self.settings.set_string(keys::COOKIE, &text).ok();
                sender.output(SettingsOutput::SaveCookie).ok();
            }
            SettingsInput::ResetSettings => {
                self.restore_on_start = true;
                self.auto_play_on_restore = false;
                self.cookie = String::new();
                sender
                    .output(SettingsOutput::UserCookieChanged(String::new()))
                    .ok();
            }
            SettingsInput::ReloadAll => {
                self.restore_on_start = self.settings.boolean(keys::RESTORE_ON_START);
                self.auto_play_on_restore = self.settings.boolean(keys::AUTO_PLAY_ON_RESTORE);
                self.cookie = self.settings.string(keys::COOKIE).to_string();
            }
        }
    }
}
