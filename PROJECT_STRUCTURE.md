# Linn 项目结构说明

## 目录结构

```
src/
├── main.rs                 # 应用程序入口
├── app.rs                  # 主应用组件（顶层窗口管理）
│
├── components/             # 可复用的 UI 组件
│   ├── mod.rs             # 模块导出
│   ├── navigation.rs      # 左侧导航栏组件
│   └── player_bar.rs      # 底部播放栏组件
│
├── pages/                  # 页面组件（右侧内容区）
│   ├── mod.rs             # 模块导出
│   ├── recommend.rs       # 为我推荐页面
│   ├── discover.rs        # 发现音乐页面
│   ├── collection.rs      # 我的收藏页面
│   └── favorites.rs       # 我喜欢的音乐页面
│
├── api/                    # API 封装层
│   ├── mod.rs             # 模块导出
│   └── ncm.rs             # 网易云音乐 API 封装
│
└── utils/                  # 工具函数
    ├── mod.rs             # 模块导出
    ├── image.rs           # 图片处理工具
    └── async_utils.rs     # 异步操作工具
```

## 模块职责

### main.rs
应用程序的入口点，负责初始化 Relm4 应用并启动主窗口。

### app.rs
主应用组件，负责：
- 管理应用窗口
- 协调导航栏、播放栏和页面组件
- 处理页面切换逻辑
- 管理应用级别的状态

### components/
可复用的 UI 组件，用于构建应用的各个部分。

#### navigation.rs
左侧导航栏组件，包含：
- 为我推荐
- 发现音乐
- 我的收藏
- 我喜欢的音乐

#### player_bar.rs
底部播放栏组件，包含：
- 封面显示
- 歌曲信息（标题/艺术家）
- 播放控制（上一首/播放暂停/下一首）
- 音量控制

### pages/
右侧内容区的不同页面组件。

#### recommend.rs
"为我推荐"页面，展示推荐歌单。

#### discover.rs
"发现音乐"页面，展示发现音乐内容。

#### collection.rs
"我的收藏"页面，展示用户收藏的内容。

#### favorites.rs
"我喜欢的音乐"页面，展示用户喜欢的歌曲列表。

### api/
API 封装层，提供统一的音乐数据访问接口。

#### ncm.rs
基于 `netease-cloud-music-api` 封装的网易云音乐 API 客户端，提供：
- 全局 API 实例
- 便捷的访问方法
- 未来可扩展更多 API 方法

### utils/
工具函数集合，提供通用功能。

#### image.rs
图片处理工具，提供：
- URL 图片加载
- 图片缓存（待实现）
- 占位符生成

#### async_utils.rs
异步操作工具，提供：
- 主线程执行回调
- 异步任务管理（待实现）

## 设计原则

1. **模块化**：每个功能模块职责单一，便于维护
2. **可复用性**：组件设计注重复用，减少代码重复
3. **声明式 UI**：使用 Relm4 的 `view!` 宏实现声明式界面
4. **类型安全**：充分利用 Rust 的类型系统确保安全性
5. **异步优先**：API 调用和耗时操作使用异步处理

## 开发流程

1. 添加新功能时，首先确定属于哪个模块
2. UI 组件优先考虑复用性，放入 `components/`
3. 独立页面放入 `pages/`
4. API 调用通过 `api/ncm.rs` 统一管理
5. 通用工具函数放入 `utils/`

## 依赖关系

```
main.rs
  └── app.rs
        ├── components/
        │     ├── navigation.rs
        │     └── player_bar.rs
        ├── pages/
        │     ├── recommend.rs
        │     ├── discover.rs
        │     ├── collection.rs
        │     └── favorites.rs
        ├── api/ncm.rs
        └── utils/
              ├── image.rs
              └── async_utils.rs
```
