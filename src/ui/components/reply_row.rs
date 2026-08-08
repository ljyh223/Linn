use relm4::gtk::prelude::*;
use relm4::prelude::*;

use crate::api::Comment;
use crate::ui::components::image::AsyncImage;

#[derive(Debug, Clone)]
pub struct ReplyRowInit {
    pub reply: Comment,
}

#[derive(Debug)]
pub struct ReplyRow {
    reply: Comment,
    mention: String,
}

#[relm4::factory(pub)]
impl FactoryComponent for ReplyRow {
    type Init = ReplyRowInit;
    type Input = ();
    type Output = ();
    type CommandOutput = ();
    type ParentWidget = gtk::Box;

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Horizontal,
            set_spacing: 10,
            set_margin_top: 6,
            set_margin_bottom: 6,

            AsyncImage {
                set_width_request: 32,
                set_height_request: 32,
                set_corner_radius: 16.0,
                set_placeholder_icon: "avatar-default-symbolic",
                set_url: format!("{}?param=64y64", self.reply.user.avatar_url),
                set_valign: gtk::Align::Start,
            },

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 4,

                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 8,

                    gtk::Label {
                        set_label: &self.reply.user.name,
                        set_halign: gtk::Align::Start,
                        add_css_class: "caption-heading",
                    },
                    gtk::Label {
                        set_label: &self.reply.time_str,
                        set_halign: gtk::Align::Start,
                        set_visible: !self.reply.time_str.is_empty(),
                        add_css_class: "caption",
                        add_css_class: "dim-label",
                    },

                    gtk::Box {
                        set_hexpand: true,
                    },

                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_spacing: 4,

                        gtk::Image {
                            set_icon_name: Some("heart-outline-thick"),
                            set_pixel_size: 12,
                            add_css_class: "dim-label",
                        },
                        gtk::Label {
                            set_label: &self.reply.liked_count.to_string(),
                            add_css_class: "caption",
                            add_css_class: "dim-label",
                        },
                    },
                },

                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_margin_bottom: 2,
                    set_visible: !self.mention.is_empty(),
                    add_css_class: "comment-quote",

                    gtk::Label {
                        set_label: &self.mention,
                        set_halign: gtk::Align::Start,
                        set_wrap: true,
                        set_xalign: 0.0,
                        add_css_class: "dim-label",
                        add_css_class: "caption",
                    },
                },

                gtk::Label {
                    set_label: &self.reply.content,
                    set_halign: gtk::Align::Start,
                    set_wrap: true,
                    set_xalign: 0.0,
                    set_selectable: true,
                },
            },
        }
    }

    fn init_model(init: Self::Init, _index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        let mention = init
            .reply
            .be_replied
            .first()
            .map(|be| format!("@{}: {}", be.user.name, be.content))
            .unwrap_or_default();
        Self {
            reply: init.reply,
            mention,
        }
    }
}
