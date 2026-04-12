# Linn

🎵 一个使用 Rust + GTK4 开发的现代某云音乐第三方桌面客户端

> ✨ 纯原生 Rust 应用，无 WebView，遵循 GNOME 桌面设计规范

---

## 项目特点

- **原生性能** - 纯 Rust + GTK4 原生渲染，无 Electron/WebView 开销
- **现代界面** - 基于 libadwaita，完整遵循 GNOME HIG 设计规范
- **MPRIS 支持** - 完整实现 MPRIS D-Bus 协议，可被系统媒体控件、锁屏界面识别
- **声明式架构** - 使用 Relm4 声明式 GUI 框架，组件化设计
- **本地缓存** - 内置图片与元数据本地缓存系统
- **模块化设计** - UI/播放器/API 完全分层，耦合度低

---

## 技术栈

| 组件 | 技术选型 |
|------|----------|
| 语言 | Rust 2024 Edition |
| UI 框架 | GTK 4 + libadwaita 1.6 |
| GUI 抽象 | Relm4 0.11 |
| 播放引擎 | GStreamer 1.26 |
| 异步运行时 | Tokio |
| API 客户端 | ncm-api-rs |
| 媒体协议 | MPRIS D-Bus |
| 缓存系统 | Moka |

---

## 已实现功能

✅ **用户界面**
- 侧边栏分栏布局 (OverlaySplitView)
- 顶部导航栏与路由系统
- 首页推荐歌单网格
- 歌单详情页面
- 异步图片加载与淡入动画
- 页面切换过渡动画

✅ **播放系统**
- 完整播放控制 (播放/暂停/上一首/下一首)
- GStreamer 音频后端
- MPRIS 桌面集成
- 播放状态管理

✅ **网络与数据**
- 网易云音乐 API 完整集成
- Cookie 认证支持
- 歌单详情、歌曲信息获取
- 播放地址解析

---

## 构建与运行

### 系统依赖 (Debian/Ubuntu)

```bash
sudo apt install \
  libgtk-4-dev \
  libadwaita-1-dev \
  libgstreamer1.0-dev \
  gstreamer1.0-plugins-base \
  gstreamer1.0-plugins-good
```

### 运行项目

```bash
# 开发运行
cargo run

# 发布构建
cargo build --release
```

---

## 项目架构

```
Linn
├──  src/player/       # 音频播放核心
│   ├── backend.rs       # GStreamer 播放后端
│   ├── player.rs        # 播放器状态管理
│   ├── mpris.rs         # MPRIS D-Bus 服务
│   └── messages.rs      # 播放器消息定义
│
├──  src/ui/           # 用户界面层
│   ├── window.rs        # 主窗口根组件
│   ├── home.rs          # 首页
│   ├── playlist_detail.rs # 歌单详情页
│   └── components/      # 可复用UI组件
│
├──  src/api/          # API 接口层
│   ├── client.rs        # API 客户端封装
│   └── mod.rs           # 数据模型定义
│
└──  src/utils/        # 通用工具
    └── utils.rs         # utils
```

---

## 开发状态

项目处于活跃开发阶段，核心架构已完成，正在逐步完善功能。

---

## 许可证

MIT License
