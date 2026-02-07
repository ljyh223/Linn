# 列表详情组件使用指南

## 概述

`ListDetail` 是一个通用的列表详情组件，用于显示歌单、专辑、艺术家等的详细信息。该组件参考了 Vue 版本的设计，提供了丰富的配置选项和交互功能。

## 组件特性

### 1. 数据结构

#### DetailData
详情数据结构，包含以下字段：
- `id`: ID
- `name`: 名称
- `cover_url`: 封面 URL
- `description`: 简介/描述
- `play_count`: 播放量
- `creator_name`: 创建者名称
- `artist_name`: 艺术家名称
- `song_count`: 歌曲数量
- `update_time`: 更新时间戳
- `create_time`: 创建时间戳
- `tags`: 标签列表
- `privacy`: 隐私状态（10 表示隐私歌单）

#### DetailType
详情类型枚举：
- `Playlist`: 歌单
- `Album`: 专辑
- `Artist`: 艺术家

#### DetailConfig
配置选项：
- `title_ellipsis`: 标题是否使用省略号
- `show_cover_mask`: 显示封面遮罩
- `show_play_count`: 显示播放量
- `show_creator`: 显示创建者
- `show_artist`: 显示艺术家
- `show_count`: 显示歌曲数量
- `show_search`: 显示搜索框
- `show_tabs`: 显示标签页

#### DetailTab
标签页类型：
- `Songs`: 歌曲
- `Comments`: 评论

### 2. 消息和输出

#### ListDetailMsg
组件消息：
- `UpdateData(Arc<DetailData>)`: 更新详情数据
- `UpdateConfig(DetailConfig)`: 更新配置
- `SearchChanged(String)`: 搜索值改变
- `PlayAllClicked`: 播放全部按钮点击
- `TabChanged(DetailTab)`: 标签页切换
- `TagClicked(String)`: 标签点击
- `DescriptionClicked`: 描述点击
- `ArtistClicked(String)`: 艺术家点击
- `ScrollingChanged(bool)`: 滚动状态改变

#### ListDetailOutput
组件输出：
- `PlayAll`: 播放全部
- `TabChanged(DetailTab)`: 标签页切换
- `SearchChanged(String)`: 搜索值改变
- `TagClicked(String)`: 标签点击
- `DescriptionClicked(String)`: 描述点击
- `ArtistClicked(u64)`: 艺术家点击

## 使用示例

### 示例 1: 直接使用 DetailData 结构

最简单的方式是直接在页面中使用 `DetailData` 结构，参考 `PlaylistDetail` 页面的实现：

```rust
use crate::components::AsyncImage;
use crate::components::list_detail::{DetailData, DetailTab};

pub struct PlaylistDetail {
    detail_data: Option<Arc<DetailData>>,
    // ...
}

// 在 view! 宏中构建 UI
gtk::Box {
    // 封面
    #[name = "cover_image"]
    AsyncImage {
        set_width_request: 240,
        set_height_request: 240,
    },

    // 标题
    #[name = "title_label"]
    gtk::Label {
        set_label: &data.name,
        add_css_class: "detail-title",
    },

    // ...
}

// 在 pre_view() 中更新数据
fn pre_view() {
    if let Some(data) = &model.detail_data {
        widgets.cover_image.set_src(Some(&data.cover_url));
        widgets.title_label.set_label(&data.name);
        // ...
    }
}
```

### 示例 2: 歌单详情页面

完整的歌单详情页面实现见 `src/pages/playlist_detail.rs`。

关键点：
1. 使用 `AsyncImage` 组件异步加载封面
2. 根据数据动态显示/隐藏元素
3. 使用 CSS 类名控制样式
4. 处理用户交互（播放全部、搜索、标签切换等）

### 示例 3: 专辑详情页面

与歌单详情类似，但配置不同：

```rust
let detail_data = DetailData {
    id: album_id,
    name: "专辑名称".to_string(),
    cover_url: "https://...".to_string(),
    description: Some("专辑描述".to_string()),
    creator_name: None,
    artist_name: Some("艺术家名称".to_string()),  // 专辑用 artist_name
    song_count: Some(12),
    // ...
};
```

### 示例 4: 艺术家详情页面

艺术家详情通常不需要显示歌曲数量和播放量：

```rust
let detail_data = DetailData {
    id: artist_id,
    name: "艺术家名称".to_string(),
    cover_url: "https://...".to_string(),
    description: Some("艺术家简介".to_string()),
    play_count: None,  // 不显示播放量
    song_count: None,  // 不显示歌曲数量
    // ...
};
```

## 样式定制

组件使用 CSS 类名来控制样式，相关样式定义在 `src/style.css` 中：

- `.list-detail`: 详情容器
- `.cover-mask`: 封面遮罩（渐变效果）
- `.play-count`: 播放量标签
- `.detail-title`: 标题
- `.detail-description`: 描述
- `.detail-meta`: 元数据标签
- `.tag-button`: 标签按钮
- `.play-button`: 播放按钮
- `.tabs`: 标签页容器
- `.tab-button`: 标签按钮
- `.list-detail.small`: 小尺寸状态（滚动时）

### 自定义样式

可以通过覆盖这些 CSS 类来自定义样式：

```css
.detail-title {
    font-size: 36px;  /* 更大的标题 */
    color: @accent_color;
}

.play-button {
    background-color: @accent_color;
    color: white;
}
```

## 工具函数

组件提供了两个工具函数：

### format_number(num: u64) -> String
格式化数字（播放量等）：
- 大于等于 1 亿：显示为 "X.X亿"
- 大于等于 1 万：显示为 "X.X万"
- 其他：显示原始数字

### format_timestamp(timestamp: i64) -> String
格式化时间戳为本地日期格式：
- 输入：Unix 时间戳（秒）
- 输出："YYYY-MM-DD" 格式

## 注意事项

1. **异步加载图片**：封面图片使用 `AsyncImage` 组件异步加载，会自动处理缓存和错误状态。

2. **数据更新**：数据更新后需要调用 `sender.input()` 发送消息，不要直接修改 model。

3. **CSS 样式**：确保应用加载了 `src/style.css`，否则组件样式不会生效。

4. **时间戳**：时间戳应该是 Unix 时间戳（秒），不是毫秒。

5. **图片 URL**：确保图片 URL 是可访问的 HTTPS 地址。

## 未来改进

- [ ] 集成真实的 API 调用
- [ ] 添加歌曲列表组件
- [ ] 添加评论组件
- [ ] 实现滚动时自动收起详情
- [ ] 添加更多交互功能（下载、收藏等）
- [ ] 支持更多自定义配置
