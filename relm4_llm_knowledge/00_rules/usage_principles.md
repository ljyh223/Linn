# Relm4 使用原则

## LLM 使用 Relm4 的核心原则

### 1. 架构理解

Relm4 是基于 Elm 架构的 GUI 框架，核心流程是：
- **Model** → 存储应用状态
- **Message** → 描述状态变更
- **Update** → 处理消息更新状态
- **View** → 根据状态渲染 UI

```rust
Message → Update → Model → View → UI
                ↑              ↓
                └──────────────┘
```

### 2. 选择合适的组件类型

根据场景选择：

| 场景 | 使用类型 |
|------|----------|
| 简单 UI 组件 | `SimpleComponent` |
| 需要后台任务 | `Component` (支持 commands) |
| 动态列表 | `FactoryComponent` |
| 异步操作 | `AsyncComponent` |
| 无 GUI 后台任务 | `Worker` |

### 3. 消息设计原则

- **Input 消息**：组件接收的指令
- **Output 消息**：向父组件报告的事件
- 使用 `#[doc(hidden)]` 标记内部消息

```rust
pub enum AppMsg {
    Increment,
    Decrement,
    #[doc(hidden)]  // 内部使用，不在文档显示
    InternalUpdate(u32),
}
```

### 4. Widget 宏使用原则

- 外层 widget 自动成为 root
- 使用 `#[watch]` 实现响应式更新
- 使用 `#[track]` 实现条件更新（需配合 tracker）
- 事件处理简化：`connect_clicked => Msg`

### 5. Factory 使用原则

- 用于动态集合的 UI 渲染
- 使用 `FactoryVecDeque` 管理数据
- 所有变更操作需要 guard
- 使用 `DynamicIndex` 跟踪移动元素

```rust
let mut guard = factory.guard();
guard.push_back(123);
// guard drop 后自动更新 UI
```

### 6. 异步处理原则

| 需求 | 方案 |
|------|------|
| 简单异步任务 | `AsyncComponent` |
| 并发处理多个消息 | `Commands` |
| CPU 密集型单线程 | `Worker` |

### 7. 状态管理

- 小型应用：直接使用 struct
- 需要跟踪变化：使用 `tracker` crate
- 复杂状态：考虑拆分子组件

### 8. 代码生成优先级

1. 优先使用 `#[relm4::component]` 宏
2. 手动实现仅在宏无法满足需求时
3. 使用 `view_output!()` 获取 widgets

### 9. Widget 引用规则

Widget 类似 `Rc`：
- clone 只增加引用计数，不创建新实例
- 自动管理生命周期
- **非线程安全**，仅限主线程使用

### 10. 避免常见错误

1. **忘记 `#[watch]`**：UI 不会更新
2. **忘记 `prelude::*`**：找不到 widget 方法
3. **在 `update` 中直接操作 widget**：违反架构原则
4. **factory 变更不用 guard**：不会触发 UI 更新
5. **异步 update 阻塞**：影响组件消息处理

### 11. Cargo.toml 依赖

```toml
[dependencies]
relm4 = "0.9"
relm4-components = "0.9"  # 可选，预置组件
tracker = "0.2"           # 可选，状态跟踪
```
