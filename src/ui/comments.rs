use relm4::factory::FactoryVecDeque;
use relm4::gtk::prelude::*;
use relm4::prelude::*;

use crate::api::{Comment, MusicComment, get_song_comments};
use crate::ui::components::image::AsyncImage;

#[derive(Debug, Clone)]
pub struct CommentRowInit {
    pub comment: Comment,
}

pub struct CommentRow {
    comment: Comment,
}

#[relm4::factory(pub)]
impl FactoryComponent for CommentRow {
    type Init = CommentRowInit;
    type Input = ();
    type Output = ();
    type CommandOutput = ();
    type ParentWidget = gtk::ListBox;

    view! {
            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_spacing: 12,
                set_margin_top: 8,
                set_margin_bottom: 8,
                set_margin_start: 8,
                set_margin_end: 8,
                set_vexpand: false,

                AsyncImage {
                    set_width_request: 40,
                    set_height_request: 40,
                    set_corner_radius: 20.0,
                    set_placeholder_icon: "folder-music-symbolic",
                    set_url: format!("{}?param=80y80", self.comment.user.avatar_url),
                    set_valign: gtk::Align::Start,
                },

                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,

                    gtk::Box{
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 4,
                        gtk::Label {
                            set_label: &self.comment.user.name,
                            set_halign: gtk::Align::Start,
                            add_css_class: "caption-heading",
                        },
                        gtk::Label {
                            set_label: &self.comment.content,
                            set_halign: gtk::Align::Start,
                            set_wrap: true,
                            set_wrap_mode: gtk::pango::WrapMode::WordChar,
                            set_selectable: true,
                            set_xalign: 0.0,
                        },
                    },

                    gtk::Box{
                        set_hexpand: true,
                    },


                    gtk::Label {
                        set_label: &format!("♥ {}", self.comment.liked_count),
                        set_halign: gtk::Align::Start,
                        add_css_class: "dim-label",
                        add_css_class: "caption",
                    },
                }
            }
        }

    fn init_model(init: Self::Init, _index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        Self {
            comment: init.comment,
        }
    }
}

#[derive(Debug)]
pub enum CommentsMsg {
    LoadComments(u64),
}

#[derive(Debug)]
pub enum CommentsOutput {}

#[derive(Debug)]
pub enum CommentsCmdMsg {
    CommentsLoaded(MusicComment),
    LoadFailed,
}

#[tracker::track]
pub struct CommentsPage {
    song_id: u64,
    is_loading: bool,
    #[do_not_track]
    hot_comments: FactoryVecDeque<CommentRow>,
    #[do_not_track]
    comments: FactoryVecDeque<CommentRow>,
}

#[relm4::component(pub)]
impl Component for CommentsPage {
    type Init = u64;
    type Input = CommentsMsg;
    type Output = CommentsOutput;
    type CommandOutput = CommentsCmdMsg;

    view! {
        #[root]
        gtk::Stack {
            set_transition_type: gtk::StackTransitionType::Crossfade,
            #[watch]
            set_visible_child_name: if model.is_loading { "loading" } else { "content" },

            add_named[Some("loading")] = &gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_halign: gtk::Align::Center,
                set_valign: gtk::Align::Center,
                set_spacing: 16,

                gtk::Spinner {
                    set_spinning: true,
                    set_width_request: 48,
                    set_height_request: 48,
                },
                gtk::Label {
                    set_label: "正在加载评论...",
                    add_css_class: "dim-label",
                }
            },

            add_named[Some("content")] = &gtk::ScrolledWindow {
                set_vexpand: true,
                set_hscrollbar_policy: gtk::PolicyType::Never,
                set_margin_start: 24,
                set_margin_end: 24,
                set_margin_top: 24,
                set_margin_bottom: 24,

                #[wrap(Some)]
                set_child = &gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 16,

                    gtk::Label {
                        set_label: "热门评论",
                        set_halign: gtk::Align::Start,
                        add_css_class: "title-4",
                    },

                    #[local_ref]
                    hot_comments_list -> gtk::ListBox {
                        add_css_class: "boxed-list",
                        add_css_class: "rich-list",
                        set_selection_mode: gtk::SelectionMode::None,
                    },

                    gtk::Separator {
                        set_margin_top: 8,
                        set_margin_bottom: 8,
                    },

                    gtk::Label {
                        set_label: "最新评论",
                        set_halign: gtk::Align::Start,
                        add_css_class: "title-4",
                    },

                    #[local_ref]
                    comments_list -> gtk::ListBox {
                        add_css_class: "boxed-list",
                        add_css_class: "rich-list",
                        set_selection_mode: gtk::SelectionMode::None,
                    },
                }
            }
        }
    }

    fn init(
        song_id: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let hot_comments = FactoryVecDeque::builder()
            .launch(gtk::ListBox::default())
            .forward(sender.input_sender(), |_| CommentsMsg::LoadComments(0));

        let comments = FactoryVecDeque::builder()
            .launch(gtk::ListBox::default())
            .forward(sender.input_sender(), |_| CommentsMsg::LoadComments(0));

        let model = Self {
            song_id,
            is_loading: true,
            hot_comments,
            comments,
            tracker: 0,
        };

        let hot_comments_list = model.hot_comments.widget();
        let comments_list = model.comments.widget();
        let widgets = view_output!();

        sender.input(CommentsMsg::LoadComments(song_id));

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        self.reset();
        match msg {
            CommentsMsg::LoadComments(id) => {
                self.set_is_loading(true);
                sender.command(move |out, _shutdown| async move {
                    match get_song_comments(id).await {
                        Ok(data) => {
                            let _ = out.send(CommentsCmdMsg::CommentsLoaded(data));
                        }
                        Err(_) => {
                            let _ = out.send(CommentsCmdMsg::LoadFailed);
                        }
                    }
                });
            }
        }
    }

    fn update_cmd(
        &mut self,
        msg: Self::CommandOutput,
        _sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        self.reset();
        match msg {
            CommentsCmdMsg::CommentsLoaded(data) => {
                {
                    let mut guard = self.hot_comments.guard();
                    guard.clear();
                    for c in data.hot_comments {
                        guard.push_back(CommentRowInit { comment: c });
                    }
                }
                {
                    let mut guard = self.comments.guard();
                    guard.clear();
                    for c in data.comments {
                        guard.push_back(CommentRowInit { comment: c });
                    }
                }
                self.set_is_loading(false);
            }
            CommentsCmdMsg::LoadFailed => {
                self.set_is_loading(false);
            }
        }
    }
}
