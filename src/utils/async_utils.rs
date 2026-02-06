//! 异步工具函数
//!
//! 提供异步操作相关的工具函数。

// TODO: 实现异步工具
// - 在 glib 主线程中执行异步操作
// - 异步任务取消
// - 超时处理

/// 在 glib 主线程中执行回调
pub fn execute_on_main_thread<F>(func: F)
where
    F: FnOnce() + Send + 'static,
{
    // TODO: 实现在主线程执行逻辑
    func();
}
