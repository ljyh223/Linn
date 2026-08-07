use std::cell::Cell;

use relm4::factory::FactoryVecDeque;
use relm4::gtk::glib::prelude::*;
use relm4::gtk::prelude::*;
use relm4::prelude::*;

use crate::api::{Comment, CommentFloor, get_comment_floor, get_song_comments_new};
use crate::ui::components::image::AsyncImage;

#[derive(Debug, Clone)]
pub struct CommentRowInit {
    pub comment: Comment,
    pub song_id: u64,
}

pub struct CommentRow {
    comment: Comment,
    song_id: u64,
    replies: Vec<Comment>,
    has_more: bool,
    expanded: bool,
    loaded: bool,
    dirty: Cell<bool>,
}

#[derive(Debug)]
pub enum CommentRowMsg {
    ToggleReplies,
}

#[derive(Debug)]
pub enum CommentRowCmd {
    RepliesLoaded(CommentFloor),
    LoadFailed,
}

fn reply_mention(reply: &Comment) -> String {
    if let Some(be) = reply.be_replied.first() {
        format!("@{}: {}", be.user.name, be.content)
    } else {
        String::new()
    }
}

fn build_reply_widget(reply: &Comment) -> gtk::Box {
    let root = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(10)
        .margin_top(6)
        .margin_bottom(6)
        .build();

    let avatar = AsyncImage::new();
    avatar.set_width_request(32);
    avatar.set_height_request(32);
    avatar.set_corner_radius(16.0);
    avatar.set_placeholder_icon("avatar-default-symbolic");
    avatar.set_valign(gtk::Align::Start);
    avatar.set_url(format!("{}?param=64y64", reply.user.avatar_url));
    root.append(&avatar);

    let text_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(4)
        .build();

    let meta = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(8)
        .build();

    let name = gtk::Label::builder()
        .label(&reply.user.name)
        .halign(gtk::Align::Start)
        .css_classes(["caption-heading"])
        .build();
    meta.append(&name);

    if !reply.time_str.is_empty() {
        let time = gtk::Label::builder()
            .label(&reply.time_str)
            .halign(gtk::Align::Start)
            .css_classes(["caption", "dim-label"])
            .build();
        meta.append(&time);
    }

    let spacer = gtk::Box::builder().hexpand(true).build();
    meta.append(&spacer);

    let like_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(4)
        .build();
    let heart = gtk::Image::builder()
        .icon_name("heart-outline-thick")
        .pixel_size(12)
        .css_classes(["dim-label"])
        .build();
    like_box.append(&heart);
    let like_count = gtk::Label::builder()
        .label(&reply.liked_count.to_string())
        .css_classes(["caption", "dim-label"])
        .build();
    like_box.append(&like_count);
    meta.append(&like_box);

    text_box.append(&meta);

    let mention = reply_mention(reply);
    if !mention.is_empty() {
        let quote = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .margin_bottom(2)
            .css_classes(["comment-quote"])
            .build();
        let quote_label = gtk::Label::builder()
            .label(&mention)
            .halign(gtk::Align::Start)
            .wrap(true)
            .xalign(0.0)
            .css_classes(["dim-label", "caption"])
            .build();
        quote.append(&quote_label);
        text_box.append(&quote);
    }

    let content = gtk::Label::builder()
        .label(&reply.content)
        .halign(gtk::Align::Start)
        .wrap(true)
        .xalign(0.0)
        .selectable(true)
        .build();
    text_box.append(&content);

    root.append(&text_box);
    root
}

fn clear_box_children(box_widget: &gtk::Box) {
    let model = box_widget.observe_children();
    let mut to_remove = Vec::new();
    for i in 0..model.n_items() {
        if let Some(child) = model.item(i) {
            to_remove.push(child);
        }
    }
    for child in to_remove {
        if let Some(widget) = child.downcast::<gtk::Widget>().ok() {
            box_widget.remove(&widget);
        }
    }
}

#[relm4::factory(pub)]
impl FactoryComponent for CommentRow {
    type Init = CommentRowInit;
    type Input = CommentRowMsg;
    type Output = ();
    type CommandOutput = CommentRowCmd;
    type ParentWidget = gtk::ListBox;

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,

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

                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 4,

                        gtk::Box {
                            set_orientation: gtk::Orientation::Horizontal,
                            set_spacing: 8,
                            gtk::Label {
                                set_label: &self.comment.user.name,
                                set_halign: gtk::Align::Start,
                                add_css_class: "caption-heading",
                            },
                            gtk::Label {
                                set_label: &self.comment.time_str,
                                set_halign: gtk::Align::Start,
                                add_css_class: "caption",
                                add_css_class: "dim-label",
                            },
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

                    gtk::Box {
                        set_hexpand: true,
                    },

                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_spacing: 4,
                        set_valign: gtk::Align::End,

                        gtk::Image {
                            set_icon_name: Some("heart-outline-thick"),
                            set_pixel_size: 14,
                            add_css_class: "dim-label",
                        },
                        gtk::Label {
                            set_label: &self.comment.liked_count.to_string(),
                            set_halign: gtk::Align::Start,
                            add_css_class: "dim-label",
                            add_css_class: "caption",
                        },
                    },
                },
            },

            #[name(toggle)]
            gtk::Button {
                set_halign: gtk::Align::Start,
                set_margin_start: 60,
                set_margin_top: 0,
                set_margin_bottom: 8,
                set_visible: self.comment.reply_count > 0,
                set_label: &format!("查看 {} 条回复", self.comment.reply_count),
                add_css_class: "comment-toggle",
                connect_clicked[sender] => move |_| {
                    sender.input(CommentRowMsg::ToggleReplies);
                },
            },

            #[name(revealer)]
            gtk::Revealer {
                set_transition_type: gtk::RevealerTransitionType::SlideDown,
                set_transition_duration: 150,
                set_reveal_child: false,

                #[wrap(Some)]
                set_child = &gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_margin_start: 24,
                    set_margin_end: 12,
                    set_margin_bottom: 8,

                    #[name(replies_box)]
                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 0,
                    }
                }
            }
        }
    }

    fn init_model(init: Self::Init, _index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        Self {
            comment: init.comment,
            song_id: init.song_id,
            replies: Vec::new(),
            has_more: false,
            expanded: false,
            loaded: false,
            dirty: Cell::new(false),
        }
    }

    fn update(&mut self, message: Self::Input, sender: FactorySender<Self>) {
        match message {
            CommentRowMsg::ToggleReplies => {
                if self.expanded {
                    self.expanded = false;
                    self.dirty.set(true);
                    return;
                }
                self.expanded = true;
                self.dirty.set(true);
                if self.loaded {
                    return;
                }
                let song_id = self.song_id;
                let comment_id = self.comment.id;
                let time = self.comment.time;
                sender.command(move |out, _shutdown| async move {
                    match get_comment_floor(song_id, comment_id, time).await {
                        Ok(floor) => {
                            let _ = out.send(CommentRowCmd::RepliesLoaded(floor));
                        }
                        Err(_) => {
                            let _ = out.send(CommentRowCmd::LoadFailed);
                        }
                    }
                });
            }
        }
    }

    fn update_cmd(&mut self, message: Self::CommandOutput, _sender: FactorySender<Self>) {
        match message {
            CommentRowCmd::RepliesLoaded(floor) => {
                self.replies = floor.replies;
                self.has_more = floor.has_more;
                self.loaded = true;
                self.dirty.set(true);
            }
            CommentRowCmd::LoadFailed => {
                self.loaded = true;
                self.expanded = false;
                self.dirty.set(true);
            }
        }
    }

    fn post_view() {
        if self.dirty.get() {
            let expanded = self.expanded;
            clear_box_children(&widgets.replies_box);
            for reply in &self.replies {
                widgets.replies_box.append(&build_reply_widget(reply));
            }
            widgets.revealer.set_reveal_child(expanded && !self.replies.is_empty());
            let label = if expanded {
                "收起回复".to_string()
            } else {
                format!("查看 {} 条回复", self.comment.reply_count)
            };
            widgets.toggle.set_label(&label);
            self.dirty.set(false);
        }
    }
}

#[derive(Debug)]
pub enum CommentsMsg {
    LoadComments(u64),
    SetSort(CommentsSort),
    LoadNextPage,
}

#[derive(Debug)]
pub enum CommentsOutput {}

#[derive(Debug)]
pub enum CommentsCmdMsg {
    CommentsLoaded(Vec<Comment>, CommentsSort, bool, String),
    NextPageLoaded(Vec<Comment>, bool, String),
    LoadFailed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommentsSort {
    /// 热门评论（sortType=2）
    Hot,
    /// 最新评论（sortType=3）
    Latest,
}

impl CommentsSort {
    fn sort_type(&self) -> i64 {
        match self {
            CommentsSort::Hot => 2,
            CommentsSort::Latest => 3,
        }
    }

    fn title(&self) -> &'static str {
        match self {
            CommentsSort::Hot => "热门评论",
            CommentsSort::Latest => "最新评论",
        }
    }
}

#[tracker::track]
pub struct CommentsPage {
    song_id: u64,
    is_loading: bool,
    sort: CommentsSort,
    #[do_not_track]
    is_loading_more: bool,
    #[do_not_track]
    has_more: bool,
    #[do_not_track]
    page_no: i64,
    #[do_not_track]
    cursor: String,
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

            #[name(scrolled)]
            add_named[Some("content")] = &gtk::ScrolledWindow {
                set_vexpand: true,
                set_hscrollbar_policy: gtk::PolicyType::Never,
                set_margin_start: 24,
                set_margin_end: 24,
                set_margin_top: 16,
                set_margin_bottom: 24,

                #[wrap(Some)]
                set_child = &gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 8,

                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_spacing: 4,

gtk::ToggleButton {
                        #[watch]
                        set_active: model.sort == CommentsSort::Hot,
                        set_label: "热门",
                        add_css_class: "flat",
                        add_css_class: "comment-sort-btn",
                        connect_clicked[sender] => move |_| {
                            sender.input(CommentsMsg::SetSort(CommentsSort::Hot));
                        },
                    },
                    gtk::ToggleButton {
                        #[watch]
                        set_active: model.sort == CommentsSort::Latest,
                        set_label: "最新",
                        add_css_class: "flat",
                        add_css_class: "comment-sort-btn",
                        connect_clicked[sender] => move |_| {
                            sender.input(CommentsMsg::SetSort(CommentsSort::Latest));
                        },
                    },
                    },

                    gtk::Label {
                        #[watch]
                        set_label: model.sort.title(),
                        set_halign: gtk::Align::Start,
                        add_css_class: "title-4",
                    },

                    #[local_ref]
                    comments_list -> gtk::ListBox {
                        add_css_class: "boxed-list",
                        set_selection_mode: gtk::SelectionMode::None,
                        set_show_separators: true,
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
        let comments = FactoryVecDeque::builder()
            .launch(gtk::ListBox::default())
            .forward(sender.input_sender(), |_| CommentsMsg::LoadComments(0));

        let model = Self {
            song_id,
            is_loading: true,
            sort: CommentsSort::Hot,
            is_loading_more: false,
            has_more: false,
            page_no: 1,
            cursor: String::new(),
            comments,
            tracker: 0,
        };

        let comments_list = model.comments.widget();
        let widgets = view_output!();

        // 滚动到接近底部时触发分页加载（无感滑动）
        let scroll_sender = sender.input_sender().clone();
        widgets.scrolled.vadjustment().connect_value_changed(move |adj| {
            let value = adj.value();
            let upper = adj.upper();
            let page_size = adj.page_size();
            if upper > 0.0 && upper - (value + page_size) < 200.0 {
                let _ = scroll_sender.send(CommentsMsg::LoadNextPage);
            }
        });

        sender.input(CommentsMsg::LoadComments(song_id));

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        self.reset();
        match msg {
            CommentsMsg::SetSort(sort) => {
                if self.sort == sort {
                    return;
                }
                self.set_sort(sort);
                self.set_is_loading(true);
                self.page_no = 1;
                self.has_more = false;
                self.is_loading_more = false;
                self.cursor.clear();
                let id = self.song_id;
                let sort_type = sort.sort_type();
                sender.command(move |out, _shutdown| async move {
                    match get_song_comments_new(id, 1, sort_type, "").await {
                        Ok((comments, has_more, cursor)) => {
                            let _ = out.send(CommentsCmdMsg::CommentsLoaded(
                                comments, sort, has_more, cursor,
                            ));
                        }
                        Err(_) => {
                            let _ = out.send(CommentsCmdMsg::LoadFailed);
                        }
                    }
                });
            }
            CommentsMsg::LoadComments(id) => {
                self.set_is_loading(true);
                self.page_no = 1;
                self.has_more = false;
                self.is_loading_more = false;
                self.cursor.clear();
                let id_clone = id;
                let sort_type = self.sort.sort_type();
                let sort = self.sort;
                sender.command(move |out, _shutdown| async move {
                    match get_song_comments_new(id_clone, 1, sort_type, "").await {
                        Ok((comments, has_more, cursor)) => {
                            let _ = out.send(CommentsCmdMsg::CommentsLoaded(
                                comments, sort, has_more, cursor,
                            ));
                        }
                        Err(_) => {
                            let _ = out.send(CommentsCmdMsg::LoadFailed);
                        }
                    }
                });
            }
            CommentsMsg::LoadNextPage => {
                if self.is_loading || self.is_loading_more || !self.has_more {
                    return;
                }
                self.is_loading_more = true;
                self.page_no += 1;
                let id = self.song_id;
                let page_no = self.page_no;
                let sort_type = self.sort.sort_type();
                let cursor = self.cursor.clone();
                log::info!(
                    "无感分页(评论): 触发加载更多, page={page_no}, sort={sort_type}, cursor={cursor:?}"
                );
                sender.command(move |out, _shutdown| async move {
                    match get_song_comments_new(id, page_no, sort_type, &cursor).await {
                        Ok((comments, has_more, next_cursor)) => {
                            let _ = out.send(CommentsCmdMsg::NextPageLoaded(
                                comments, has_more, next_cursor,
                            ));
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
            CommentsCmdMsg::CommentsLoaded(comments, sort, has_more, cursor) => {
                if self.sort != sort {
                    return;
                }
                {
                    let mut guard = self.comments.guard();
                    guard.clear();
                    for c in comments {
                        guard.push_back(CommentRowInit {
                            comment: c,
                            song_id: self.song_id,
                        });
                    }
                }
                self.has_more = has_more;
                self.cursor = cursor;
                self.set_is_loading(false);
            }
            CommentsCmdMsg::NextPageLoaded(comments, has_more, cursor) => {
                self.is_loading_more = false;
                log::info!(
                    "无感分页(评论): 已加载 {} 条, 当前第 {} 页, has_more={}, next_cursor={:?}",
                    comments.len(),
                    self.page_no,
                    has_more,
                    cursor,
                );
                {
                    let mut guard = self.comments.guard();
                    for c in comments {
                        guard.push_back(CommentRowInit {
                            comment: c,
                            song_id: self.song_id,
                        });
                    }
                }
                self.has_more = has_more;
                self.cursor = cursor;
            }
            CommentsCmdMsg::LoadFailed => {
                self.is_loading_more = false;
                self.set_is_loading(false);
            }
        }
    }
}