# Relm4 LLM 知识库

为 LLM 提供的 Relm4 GUI 框架精炼知识文档。

## 目录结构

```
relm4_llm_knowledge/
├── 00_rules/
│   └── usage_principles.md      # 核心使用原则
│
├── 01_core_concepts/
│   ├── component.md             # Component 组件基础
│   ├── model_msg.md             # Model 和 Message
│   ├── update_flow.md           # 更新流程
│   ├── view_macro.md            # view! 宏详解
│   └── widgets.md               # Widget 使用
│
├── 02_component_types/
│   ├── component.md             # SimpleComponent vs Component
│   ├── factory_component.md     # FactoryComponent 动态列表
│   ├── async_component.md       # AsyncComponent 异步组件
│   └── worker.md                # Worker 无 GUI 组件
│
├── 03_patterns/
│   ├── list_factory.md          # 列表 Factory 模式
│   ├── child_components.md      # 子组件模式
│   ├── commands_async.md        # Commands 异步模式
│   └── tracker_pattern.md       # Tracker 跟踪模式
│
├── 04_examples/
│   ├── simple.rs                # 简单计数器示例
│   ├── factory.rs               # 动态列表示例
│   ├── async.rs                 # 异步组件示例
│   └── worker.rs                # Worker 示例
│
└── 90_ignored/
    ├── migrations.md            # 版本迁移（忽略）
    ├── cli.md                   # CLI 工具（忽略）
    └── ci.md                    # CI 配置（忽略）
```

## 快速参考

### 基本组件模板

```rust
use relm4::prelude::*;

#[derive(Debug)]
pub enum AppMsg {
    Increment,
}

pub struct AppModel {
    counter: u32,
}

#[relm4::component]
impl SimpleComponent for App {
    type Init = u32;
    type Input = AppMsg;
    type Output = ();
    type Root = gtk::Window;

    fn init_root() -> Self::Root {
        gtk::Window::default()
    }

    fn init(
        value: Self::Init,
        root: &Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = AppModel { counter: value };
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    view! {
        gtk::Window {
            set_title: Some("App"),
            gtk::Label {
                #[watch]
                set_label: &self.model.counter.to_string(),
            }
        }
    }

    fn update(&mut self, msg: Self::Input, _: ComponentSender<Self>) {
        match msg {
            AppMsg::Increment => self.model.counter += 1,
        }
    }
}
```

### 选择组件类型

| 需求 | 使用 |
|------|------|
| 简单 UI | `SimpleComponent` |
| 后台任务 | `Component` (commands) |
| 动态列表 | `FactoryComponent` |
| 异步初始化 | `AsyncComponent` |
| 后台计算 | `Worker` |

### 依赖配置

```toml
[dependencies]
relm4 = "0.9"
relm4-components = "0.9"  # 可选
tracker = "0.2"           # 可选，状态跟踪
```

## 关键概念

### Elm 架构

```
用户交互 → Message → Update → Model → View → UI
                ↑                           ↓
                └───────────────────────────┘
```

### Widget 特性

- 类似 `Rc`，clone 增加引用
- 自动管理生命周期
- 非线程安全，仅主线程

### Factory 模式

- 用于动态集合
- 使用 `FactoryVecDeque`
- 所有变更需要 guard
- 自动优化 UI 更新

## 使用建议

1. 优先使用宏简化代码
2. 使用 `#[watch]` 实现响应式更新
3. 复杂状态考虑子组件
4. 动态列表使用 Factory
5. 后台任务使用 Commands 或 Worker

## 版本

基于 Relm4 0.9+ 文档整理。
