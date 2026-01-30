use iced::Color;

pub struct Colors;

impl Colors {
    // 侧边栏颜色
    pub const SIDEBAR_BG: Color = Color {
        r: 0.12,
        g: 0.12,
        b: 0.15,
        a: 1.0,
    };

    // 主内容区背景
    pub const MAIN_BG: Color = Color {
        r: 0.08,
        g: 0.08,
        b: 0.1,
        a: 1.0,
    };

    // 未选中按钮背景
    pub const BUTTON_INACTIVE_BG: Color = Color {
        r: 0.15,
        g: 0.15,
        b: 0.18,
        a: 1.0,
    };

    // 选中按钮背景
    pub const BUTTON_ACTIVE_BG: Color = Color {
        r: 0.2,
        g: 0.2,
        b: 0.25,
        a: 1.0,
    };

    // 未选中按钮文字
    pub const TEXT_INACTIVE: Color = Color {
        r: 0.8,
        g: 0.8,
        b: 0.85,
        a: 1.0,
    };

    // 选中按钮文字
    pub const TEXT_ACTIVE: Color = Color {
        r: 0.3,
        g: 0.8,
        b: 0.9,
        a: 1.0,
    };

    // Logo 颜色
    pub const LOGO: Color = Color {
        r: 0.3,
        g: 0.8,
        b: 0.9,
        a: 1.0,
    };

    // 描述文字
    pub const TEXT_DESCRIPTION: Color = Color {
        r: 0.6,
        g: 0.6,
        b: 0.65,
        a: 1.0,
    };

    // 透明
    pub const TRANSPARENT: Color = Color::TRANSPARENT;
}
