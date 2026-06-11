# Linn 代码质量分析与重构计划

## 当前架构

```
┌─────────────────────────────────────────────────────────────────┐
│                          Window                                 │
│  - 路由管理    - 全屏歌词管理  - 侧栏状态  - Dialog  - Toast     │
└────────────┬────────────────────────────────────────────────────┘
             │
     ┌───────▼────────┐        ┌─────────────────┐
     │     Sidebar     │        │ FullscreenLyric  │
     │  ┌───────────┐  │        │ (动态创建，从    │
     │  │PlayerPage │  │        │  EventBus 订阅)  │
     │  │LyricPage  │  │        └─────────────────┘
     │  │QueuePage  │  │
     │  └───────────┘  │
     └────────┬─────────┘
              │
              │ PlayerEvent (通过 EventBus 广播)
     ┌────────▼─────────┐
     │  PlayerEventBus  │ ← 多播路由器
     └────────┬─────────┘
              │
     ┌────────▼─────────┐
     │  PlayerFacade    │  ← 独立线程
     │  ├─ GstEngine    │
     │  ├─ QueueManager │
     │  └─ MPRIS        │
     └──────────────────┘
```

## 已识别问题

### 问题 1: `window.rs` 是上帝对象（713 行） ✅ 已优化

`Window` 结构体承担了太多职责：
- 路由管理（`history`、`current_route`、`render_current_route`）
- ~~播放器事件转发（手动 match 每个 `PlayerEvent` 再 emit 给 sidebar/fullscreen）~~
- 侧栏状态管理（`sidebar_state`、`apply_sidebar_state`）
- 全屏歌词管理（`fullscreen_lyric`、`open/close_fullscreen_lyric`）
- 用户信息管理、Dialog 管理

**已优化**: 通过 `PlayerEventBus` 实现事件自动广播，Window 不再手动转发事件给 Sidebar。

### 问题 2: `PlayerPageOutput` 重复翻译 ✅ 已解决

~~同一个 `PlayerPageOutput` 被翻译了**两次**：~~
1. ~~`sidebar.rs:164-248`：把 `PlayerPageOutput` 包成 `SidebarOutput::PlayerCommand`~~
2. ~~`window.rs:205-246`：再把 `SidebarOutput::PlayerCommand` 翻译成 `WindowMsg::SendCommandToPlayer`~~

**已解决**: `SidebarOutput` 现在直接输出 `PlayerCommand` 和 UI 操作，Window 直接处理 `PlayerCommand`。

### 问题 3: 播放器页面 `ui/player.rs` 里的 `progress_scale` hack

用 `Rc<Cell<bool>>` 的 `is_seeking` 来防止信号循环，这是典型的"状态同步用 hack 补丁"模式。

### 问题 4: 错误处理不一致

- 调试输出混用 `eprintln!` 和 `log::error!`
- 有些地方 silent fail，有些 panic
- 有调试遗留代码

### 问题 5: 重复的 UI 模式没有抽象

- 加载状态（`is_loading` + Stack 切换）
- 横向滚动列表 + 左右箭头
- 歌曲列表 + TrackRow 管理

### 问题 6: 魔法数字散布

尺寸、间距等硬编码数字散布各处。

### 问题 7: `utils/lyric_parse.rs` 1308 行

歌词解析逻辑过于庞大。

---

## 重构计划

### 阶段 1: 消除消息翻译层 ✅ 已完成

**目标**: `Sidebar` 输出直接是 `PlayerCommand`，去掉 `PlayerPageOutput → SidebarOutput → WindowMsg` 的双重翻译。

**改动**:
1. `sidebar.rs`：`SidebarOutput` 直接使用 `PlayerCommand`，不再包一层
2. `window.rs`：`WindowMsg` 里使用 `PlayerCommandReceived`，直接处理 `PlayerCommand`
3. `PlayerPageOutput` 保留用于 PlayerPage 内部，Sidebar 负责转换

**收益**: 改命令时只改一处，减少 ~80 行重复 match 代码。

### 阶段 2: 消除 Window 的事件转发 ✅ 已完成

**目标**: `PlayerFacade` 的事件直接分发到需要的组件，Window 不再手动转发。

**方案**: 引入 `PlayerEventBus`，让多个组件订阅 PlayerEvent。

**改动**:
1. 创建 `PlayerEventBus`（`player/event_router.rs`）：持有多个 `Sender<PlayerEvent>`，广播事件
2. `PlayerFacade` 改为发送 `PlayerEvent` 而不是 `WindowMsg`
3. `PlayerEvent` 增加 `ShowToast` 变体，替代原来的 `WindowMsg::ShowToast`
4. `Sidebar` 和 `Window` 在初始化时从 `PlayerEventBus` 订阅事件
5. `FullscreenLyricPage` 仍然由 Window 转发（因为是动态创建的）

**收益**: Window 减少 ~50 行代码，新增事件时只需在消费者端处理。

---

## 待记录问题

- [x] `progress_scale` hack ✅ 已解决
- [ ] 错误处理统一（阶段 3）
- [x] UI 组件抽象 ✅ 已解决
- [ ] 魔法数字提取（阶段 4）
- [ ] 歌词解析拆分（阶段 5）

---

## 问题解决方案

### 问题 3: `progress_scale` hack ✅ 已解决

**原问题**: 用 `Rc<Cell<bool>>` 的 `is_seeking` 来防止信号循环，这是典型的"状态同步用 hack 补丁"模式。

**解决方案**: 使用 GTK 的信号阻塞机制（`block_signal`/`unblock_signal`）。

**改动**:
1. `player.rs`：删除 `Rc<Cell<bool>>` 和 `is_seeking` 字段
2. 使用 `Component` trait 替代 `SimpleComponent`，以便在 `update` 中访问 `widgets`
3. 存储 `progress_scale` 和 `seek_handler_id`，在更新进度时阻塞信号

**收益**: 代码更清晰，消除了 hack 模式，使用 GTK 原生机制。

### 问题 5: UI 组件抽象 ✅ 已解决

**原问题**: 横向滚动列表 + 左右箭头的模式在 `home.rs` 中重复出现。

**解决方案**: 创建 `ScrollableRow` 组件封装横向滚动列表。

**改动**:
1. 新增 `components/scrollable_row.rs`：封装标题、左右箭头、滚动窗口
2. `home.rs`：使用 `ScrollableRow` 替代手动创建的滚动区域
3. 删除 `ScrollLeft`、`ScrollRight`、`ScrollHomeLeft`、`ScrollHomeRight` 消息和处理逻辑

**收益**: 代码更简洁，滚动逻辑可复用，减少了 ~50 行重复代码。
