# linn 开发文档

## 技术栈

- Rust + GTK4 (gtk4-rs 0.11) + libadwaita (adw 0.7) + relm4 0.11
- GStreamer 音频引擎 + MPRIS D-Bus 集成
- Tokio 异步运行时 + flume 通道
- 参考项目: Tonearm (Go+GTK4), ratic (Rust+relm4)

## 组件架构

```
Window (ApplicationWindow)
  └── Header (OverlaySplitView)          ← 根布局，拆分侧边栏与内容
        ├── Sidebar (ToolbarView)         ← 侧边栏：播放器/歌词/队列
        └── Content (ToolbarView)         ← 内容区：首页/探索/收藏/歌手详情
```

每个子组件是一个独立的 `SimpleComponent` 或 `Component`，有自己的 `view!`、`init()`、`update()`。

### 组件类型选择

- **SimpleComponent**: 适用于纯 UI 组件，无异步操作
- **Component**: 需要异步命令处理（`CommandOutput`）时使用，如网络请求、数据加载

## 如何创建自定义组件

### 1. 基本模板

```rust
use log::trace;
use relm4::gtk::prelude::*;           // GTK trait 方法
use relm4::prelude::*;                // SimpleComponent, ComponentSender 等
use relm4::{adw, ComponentParts, gtk};

pub struct MyComponent {
    // 存放需要在 update() 中访问的 widget 引用
    // 以及业务状态
}

#[derive(Debug)]
pub enum MyMsg {
    // 输入消息 — 用户交互触发
}

#[derive(Debug)]
pub enum MyOutput {
    // 输出消息 — 通知父组件（没有可设为 ()）
}

#[relm4::component(pub)]
impl SimpleComponent for MyComponent {
    type Init = ();                    // 初始化参数
    type Input = MyMsg;               // 输入消息
    type Output = MyOutput;           // 输出消息

    view! {
        #[root]
        adw::ToolbarView {            // 根 widget
            // 声明式定义 widget 树
        }
    }

    fn init(
        _init: Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let mut model = Self { /* ... */ };
        let mut widgets = view_output!();
        // 动态添加子 widget
        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>) {
        trace!("MyComponent: {message:?}");
        // 处理消息，更新 model 和 widget
    }
}
```

### 2. 异步组件模板 (Component)

```rust
use relm4::{Component, ComponentParts, ComponentSender, gtk};

pub struct AsyncPage {
    data: Vec<Item>,
}

#[derive(Debug)]
pub enum AsyncPageMsg {
    LoadData(u64),
    ItemClicked(u64),
}

#[derive(Debug)]
pub enum AsyncPageCmdMsg {
    DataLoaded(Vec<Item>),
}

#[derive(Debug)]
pub enum AsyncPageOutput {
    Navigate(u64),
}

#[relm4::component(pub)]
impl Component for AsyncPage {
    type Init = u64;
    type Input = AsyncPageMsg;
    type Output = AsyncPageOutput;
    type CommandOutput = AsyncPageCmdMsg;

    view! {
        #[root]
        gtk::Box {
            // widget 树
        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Self { data: Vec::new() };
        let widgets = view_output!();

        // 触发初始加载
        sender.input(AsyncPageMsg::LoadData(init));
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match msg {
            AsyncPageMsg::LoadData(id) => {
                // 异步命令：在后台线程执行
                sender.command(move |out, _shutdown| async move {
                    match fetch_data(id).await {
                        Ok(data) => {
                            let _ = out.send(AsyncPageCmdMsg::DataLoaded(data));
                        }
                        Err(e) => log::error!("Failed to load: {}", e),
                    }
                });
            }
            AsyncPageMsg::ItemClicked(id) => {
                sender.output(AsyncPageOutput::Navigate(id)).unwrap();
            }
        }
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            AsyncPageCmdMsg::DataLoaded(data) => {
                self.data = data;
            }
        }
    }
}
```

### 3. 在父组件中使用子组件

```rust
// header.rs 中使用 Content 子组件
pub struct Header {
    content: Controller<Content>,      // 存储子组件的 Controller
}

fn init(..., sender: ComponentSender<Self>) -> ComponentParts<Self> {
    // 方式 A: 不需要接收输出
    let sidebar = Sidebar::builder().launch(()).detach();

    // 方式 B: 需要转发输出消息
    let content = Content::builder().launch(()).forward(
        sender.input_sender(),
        |output| match output {
            ContentOutput::ToggleSidebar => HeaderMsg::ToggleSidebar,
        },
    );

    let model = Self { sidebar, content, /* ... */ };
    let widgets = view_output!();
    ComponentParts { model, widgets }
}
```

在 `view!` 中引用子组件的 widget：

```rust
view! {
    #[root]
    adw::OverlaySplitView {
        set_sidebar: Some(model.sidebar.widget()),   // 子组件的根 widget
        #[wrap(Some)]
        set_content = model.content.widget(),
    }
}
```

## 关键设计模式

### FactoryVecDeque — 动态列表

用于创建动态数量的子组件列表，如歌单卡片、歌曲列表等：

```rust
use relm4::prelude::FactoryVecDeque;

pub struct MyPage {
    items: FactoryVecDeque<ItemCard>,
}

#[relm4::component(pub)]
impl Component for MyPage {
    // ...

    fn init(..., sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let mut model = Self {
            // 先用占位 FlowBox 创建
            items: FactoryVecDeque::builder()
                .launch(FlowBox::default())
                .forward(sender.input_sender(), |output| {
                    MyPageMsg::ItemAction(output)
                }),
        };

        let widgets = view_output!();

        // 用真实 widget 重新创建
        model.items = FactoryVecDeque::builder()
            .launch(widgets.flow_box.clone())
            .forward(sender.input_sender(), |output| {
                MyPageMsg::ItemAction(output)
            });

        ComponentParts { model, widgets }
    }

    fn update_cmd(&mut self, msg: Self::CommandOutput, ...) {
        match msg {
            MyPageCmdMsg::DataLoaded(items) => {
                let mut guard = self.items.guard();
                guard.clear();
                for item in items {
                    guard.push_back(ItemCardInit { /* ... */ });
                }
            }
        }
    }
}
```

### `#[name]` 属性 — 在 init() 中访问 view! 中的 widget

`view!` 宏在 `init()` 返回前执行（`view_output!()` 调用时），但有时需要在 `init()` 中向
widget 动态添加子元素（比如循环创建按钮）。用 `#[name = "xxx"]` 或 `#[name(xxx)]` 标记
widget，之后通过 `widgets.xxx` 访问：

```rust
view! {
    #[root]
    adw::ToolbarView {
        #[name(stack)]                    // 命名这个 ViewStack
        #[wrap(Some)]
        set_content = &adw::ViewStack {},

        #[name(footer)]                   // 命名这个 Box
        add_bottom_bar = &gtk::Box { ... },
    }
}

fn init(...) -> ComponentParts<Self> {
    let mut model = Self { /* ... */ };
    let mut widgets = view_output!();     // 执行 view!，生成命名 widget

    // 现在可以访问 widgets.stack 和 widgets.footer
    widgets.stack.add_titled(&page, Some("home"), "Home");
    widgets.footer.append(&btn);

    ComponentParts { model, widgets }
}
```

### model 变量必须在 view_output!() 之前声明

`#[relm4::component]` 宏需要在 `view_output!()` 调用之前找到名为 `model` 的变量。
如果 model 的字段依赖于 view! 中的 widget（比如按钮列表），可以先用占位值初始化，
在 `view_output!()` 之后再填充：

```rust
let mut model = Self {
    stack: adw::ViewStack::default(),   // 占位
    buttons: Vec::new(),
};
let mut widgets = view_output!();

model.stack = widgets.stack.clone();     // 用真实 widget 替换占位值
// 循环创建按钮并 push 到 model.buttons
```

### 按钮选中状态 — CSS class 切换

参考 Tonearm 的做法，使用 CSS class `raised`/`flat` 实现按钮的选中/未选中效果：

```rust
// 初始化时
if tag == "home" {
    btn.add_css_class("raised");        // 选中
} else {
    btn.add_css_class("flat");          // 未选中
}

// 切换时
for btn in &self.buttons {
    btn.remove_css_class("raised");
    btn.add_css_class("flat");
}
if let Some(btn) = self.buttons.get(idx) {
    btn.remove_css_class("flat");
    btn.add_css_class("raised");
}
```

### 路由系统

使用 `AppRoute` 枚举管理页面导航：

```rust
#[derive(Debug, Clone, PartialEq, Display)]
pub enum AppRoute {
    Home,
    Explore,
    Collection,
    PlaylistDetail(PlaylistType),
    Artist(u64),
}

// 在父组件中根据路由切换内容
match route {
    AppRoute::Home => { /* 显示首页 */ },
    AppRoute::PlaylistDetail(PlaylistType::Playlist(id)) => { /* 显示歌单详情 */ },
    AppRoute::Artist(id) => { /* 显示歌手页面 */ },
}
```

### 组件间通信

子组件通过 `Output` 消息向父组件通信，父组件通过 `.forward()` 接收：

```
Content (子组件)                    Header (父组件)
    │                                  │
    ├─ Output::ToggleSidebar  ───────> ├─ Input::ToggleSidebar
    │                                  │
    └─ sender.output(...)             └─ forward(sender.input_sender(), |out| ...)
```

如果子组件需要接收父组件的消息，使用 `Controller::emit()` 发送 `Input` 消息。

## 播放器架构

播放器采用 **门面模式 (Facade)**，通过 `PlayerFacade` 统一管理：

```
UI (WindowMsg)
    │
    ├─ PlayerCommand ──────> PlayerFacade
    │                          ├─ QueueManager (队列管理)
    │                          ├─ GstEngine (GStreamer 引擎)
    │                          └─ MPRIS (D-Bus)
    │
    └─ PlayerEvent <───────────┘
```

### PlaySource 类型

```rust
pub enum PlaySource {
    // 懒加载：已有部分 tracks，后续靠 ids 加载完整信息
    LazyQueue { tracks, track_ids, playlist },
    // 通过 ID 加载：歌单/专辑/每日推荐
    ById(PlaylistType),
    // 直接使用完整 tracks
    DirectTracks(Arc<Vec<Song>>),
    // 歌手热门歌曲
    ArtistQueue { songs, artist_name, artist_id },
}
```

### 异步播放流程

1. UI 发送 `PlayerCommand::Play { source, start_index }`
2. `PlayerFacade` 根据 source 类型处理：
   - `LazyQueue`: 直接加载到队列，开始播放
   - `ById`: 异步获取歌单/专辑详情，再加载到队列
   - `ArtistQueue`: 直接加载歌手歌曲到队列
3. 播放时异步获取歌曲 URL
4. 通过 `PlayerEvent` 通知 UI 更新

## 踩坑记录

### 1. `view!` 宏的执行时机

`view!` 宏（通过 `view_output!()` 调用）在 `init()` 函数体中执行，但它生成的
widget 会覆盖 `init()` 参数 `root` 的内容。这意味着**不能**在 `init()` 中先往
`root` 添加子 widget 再调用 `view_output!()` — 那些子 widget 会被覆盖。

正确做法：在 `view!` 中声明所有静态 widget 结构，用 `#[name]` 暴露需要动态操作的
widget，在 `init()` 中通过 `widgets.xxx` 动态添加子元素。

### 2. `builder()` 方法找不到

`SimpleComponent::builder()` 需要 `SimpleComponent` trait 在作用域。必须导入：

```rust
use relm4::prelude::*;    // 推荐，导入所有常用 trait
// 或者
use relm4::SimpleComponent;
```

注意：`relm4::Component` 和 `relm4::SimpleComponent` 是不同的 trait。
`Component` 提供 `builder()` 以及更多功能（CommandOutput 等），
`SimpleComponent` 也提供 `builder()` 但更轻量。

父组件调用 `SubComponent::builder()` 时，如果报 "no function named builder"，
说明 SubComponent 没有正确实现 `SimpleComponent`（通常是宏没有展开）。

### 3. `#[relm4::component]` 宏展开失败 — "unable to determine model name"

这个错误会导致宏停止展开，连锁产生 `Root`、`init_root`、`view_output` 都找不到。
原因是在 `view_output!()` 调用之前没有一个名为 `model` 的变量。

```rust
// 错误 — 宏找不到 model
let widgets = view_output!();
let model = Self { ... };

// 正确
let model = Self { ... };
let widgets = view_output!();
```

如果 model 需要可变，用 `let mut model = ...`。

### 4. `init()` 中的 `model` 必须可变才能 push 按钮

如果你在 `init()` 中循环创建按钮并 push 到 `model.buttons`，model 必须声明为 `mut`：

```rust
let mut model = Self { buttons: Vec::new(), /* ... */ };
```

### 5. 子组件 widget 引用 — `model.xxx.widget()` vs 直接引用

在 `view!` 中，通过 `model.xxx.widget()` 引用子组件的根 widget（其中 `xxx` 是
`Controller<SubComponent>` 类型）。这个方法返回的是 `&gtk::Widget`，可以直接传给
`set_sidebar`、`set_content` 等接受 `Option<Widget>` 的方法。

如果需要在 `init()` 中操作子组件，使用 Controller 的方法：`.emit(msg)` 发送消息，
`.widget()` 获取 widget 引用。

### 6. ToggleButton 信号连接

`ToggleButton` 的 `set_active` 方法需要 `ToggleButtonExt` trait：

```rust
use relm4::gtk::prelude::ToggleButtonExt;
```

在 `view!` 中用 `connect_toggled` 连接信号：

```rust
gtk::ToggleButton {
    set_active: true,
    connect_toggled[sender] => move |_| {
        sender.output(SomeOutput::Something).unwrap();
    },
}
```

### 7. `set_name` 不存在于 ViewStack

`adw::ViewStack` 没有 `set_name` 方法。如果需要给 ViewStack 设置 widget 名称，使用：

```rust
// view! 中
adw::ViewStack {
    set_widget_name: Some("my_stack"),   // WidgetExt::set_widget_name
}
```

但对于 `#[name(stack)]` 标记的 widget，不需要手动设置名称，宏已经处理好了。

### 8. FactoryVecDeque 必须在 view_output!() 之后重新初始化

`FactoryVecDeque::builder().launch(widget)` 需要真实的 FlowBox/ListBox widget。
如果在 `view_output!()` 之前创建，只能用占位 widget，之后必须用真实 widget 重建：

```rust
fn init(...) -> ComponentParts<Self> {
    let mut model = Self {
        // 占位：使用空 FlowBox
        items: FactoryVecDeque::builder()
            .launch(FlowBox::default())
            .forward(sender.input_sender(), |out| /* ... */),
    };

    let widgets = view_output!();

    // 真实初始化：使用 view! 中的 widget
    model.items = FactoryVecDeque::builder()
        .launch(widgets.flow_box.clone())
        .forward(sender.input_sender(), |out| /* ... */);

    ComponentParts { model, widgets }
}
```

### 9. Component 的 update_cmd 异步回调

`Component` trait 的 `update_cmd` 用于处理 `sender.command()` 发送的异步结果。
`command()` 闭包在 Tokio 异步运行时中执行，完成后通过 `out.send()` 发回：

```rust
fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
    match msg {
        Msg::LoadData(id) => {
            sender.command(move |out, _shutdown| async move {
                match api_call(id).await {
                    Ok(data) => { let _ = out.send(CmdMsg::Loaded(data)); }
                    Err(e) => log::error!("{}", e),
                }
            });
        }
    }
}

fn update_cmd(&mut self, msg: Self::CommandOutput, ...) {
    match msg {
        CmdMsg::Loaded(data) => { self.data = data; }
    }
}
```

### 10. Arc 用于跨线程共享数据

播放队列中的歌曲数据需要在 UI 线程和播放器线程之间共享，使用 `Arc<Vec<Song>>`：

```rust
use std::sync::Arc;

// 传递给播放器
sender.output(PlayQueue {
    songs: Arc::new(songs_vec),  // 包装为 Arc
    start_index: 0,
})
```

## 文件结构

```
src/
├── main.rs                 # 应用入口
├── api/                    # 网易云音乐 API 封装
│   ├── mod.rs
│   ├── client.rs           # HTTP 客户端初始化
│   ├── model.rs            # 数据模型 (Song, Playlist, Artist 等)
│   ├── user.rs             # 用户相关 API
│   ├── playlist.rs         # 歌单相关 API
│   ├── song.rs             # 歌曲相关 API
│   ├── album.rs            # 专辑相关 API
│   ├── artist.rs           # 歌手相关 API
│   ├── recommend.rs        # 推荐相关 API
│   └── utils.rs            # API 工具函数
│
├── player/                 # 播放器核心
│   ├── mod.rs
│   ├── engine.rs           # GStreamer 播放引擎
│   ├── player.rs           # 播放器主逻辑
│   ├── queue.rs            # 播放队列管理
│   ├── facade.rs           # 播放器门面 (统一接口)
│   ├── messages.rs         # 播放器消息定义
│   └── mpris.rs            # MPRIS D-Bus 集成
│
├── ui/                     # UI 层
│   ├── mod.rs
│   ├── window.rs           # 根窗口
│   ├── header.rs           # 布局控制器
│   ├── sidebar.rs          # 侧边栏
│   ├── home.rs             # 首页推荐
│   ├── explore.rs          # 探索页
│   ├── collection.rs       # 收藏页
│   ├── playlist_detail.rs  # 歌单详情
│   ├── artist.rs           # 歌手页面
│   ├── player.rs           # 播放器 UI
│   ├── lyric.rs            # 歌词页面
│   ├── queue.rs            # 播放队列 UI
│   ├── setting.rs          # 设置页面
│   ├── about.rs            # 关于对话框
│   ├── route.rs            # 路由系统
│   ├── model/              # UI 数据模型
│   │   ├── mod.rs
│   │   ├── playlist.rs     # PlaylistType, PlaySource, DetailView
│   │   └── lyric.rs        # LyricLine, LyricChar 等
│   └── components/         # 可复用 UI 组件
│       ├── mod.rs
│       ├── playlist_card.rs    # 歌单卡片
│       ├── track_row.rs        # 歌曲行
│       ├── lyric_widget.rs     # 歌词组件
│       ├── artist_dialog.rs    # 歌手对话框
│       ├── collect_dialog.rs   # 收藏对话框
│       ├── image/              # 异步图片组件
│       │   ├── mod.rs
│       │   ├── widget.rs
│       │   ├── imp.rs
│       │   └── image_manager.rs
│       └── artist/             # 歌手页面子组件
│           ├── mod.rs
│           ├── song_list.rs    # 歌曲列表
│           ├── album_grid.rs   # 专辑网格
│           └── mv_grid.rs      # MV 网格
│
└── utils/                  # 工具层
    ├── mod.rs
    ├── lyric_parse.rs      # 歌词解析 (LRC/YRC)
    └── utils.rs            # 通用工具函数
```

### 新增组件步骤

1. **创建 `.rs` 文件**，选择合适的组件类型：
   - 纯 UI 交互 → `SimpleComponent`
   - 需要异步操作 → `Component`（实现 `update_cmd`）

2. **在 `mod.rs` 中添加模块声明**：`pub mod xxx;`

3. **在父组件中使用**：
   ```rust
   use super::xxx::XxxComponent;

   // 存储 Controller
   pub struct ParentComponent {
       child: Controller<XxxComponent>,
   }

   // 初始化并转发输出
   fn init(..., sender: ComponentSender<Self>) -> ComponentParts<Self> {
       let child = XxxComponent::builder()
           .launch(init_data)
           .forward(sender.input_sender(), |output| {
               ParentMsg::ChildOutput(output)
           });
       // ...
   }
   ```
