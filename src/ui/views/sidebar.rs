use crate::app::Message;
use crate::pages::Page;
use crate::theme::Colors;
use iced::widget::{button, column, container, row, svg, text};
use iced::{Element, Fill, Length};

pub struct Sidebar {
    current_page: Page,
}

impl Sidebar {
    pub fn new(current_page: Page) -> Self {
        Self { current_page }
    }

    pub fn view(&self) -> Element<Message> {
        let nav_buttons = self.create_nav_buttons();

        let sidebar_content = column![
            // Logo / 标题区域
            self.view_logo(),
            // 导航按钮
            nav_buttons,
        ]
        .spacing(20);

        container(sidebar_content)
            .width(200)
            .height(Fill)
            .style(|_theme| container::Style {
                background: Some(iced::Background::Color(Colors::SIDEBAR_BG)),
                ..Default::default()
            })
            .into()
    }

    fn view_logo(&self) -> Element<Message> {
        container(
            text("Linn Music")
                .size(20)
                .style(|_theme| text::Style {
                    color: Some(Colors::LOGO),
                }),
        )
        .padding(20)
        .align_x(iced::alignment::Horizontal::Center)
        .into()
    }

    fn create_nav_buttons(&self) -> Element<Message> {
        let mut buttons = Vec::new();

        for &page in &Page::NAV_PAGES {
            let is_active = self.current_page == page;
            let button = self.create_nav_button(page, is_active);
            buttons.push(button);
        }

        column(buttons).spacing(4).into()
    }

    fn create_nav_button(&self, page: Page, is_active: bool) -> Element<Message> {
        let icon = svg(page.icon()).width(Length::Fixed(24.0)).height(Length::Fixed(24.0));

        let content = row![
            icon,
            text(page.title())
                .size(14)
                .style(move |_theme| text::Style {
                    color: Some(if is_active {
                        Colors::TEXT_ACTIVE
                    } else {
                        Colors::TEXT_INACTIVE
                    }),
                })
        ]
        .spacing(10)
        .align_y(iced::alignment::Vertical::Center);

        container(
            button(content)
                .on_press(Message::Navigate(page))
                .width(Length::Fill)
                .padding([10, 15])
                .style(move |_theme, _status| Self::button_style(is_active)),
        )
        .width(Length::Fill)
        .padding([4, 8])
        .into()
    }

    fn button_style(is_active: bool) -> button::Style {
        let (bg, text_color) = if is_active {
            (Colors::BUTTON_ACTIVE_BG, Colors::TEXT_ACTIVE)
        } else {
            (Colors::BUTTON_INACTIVE_BG, Colors::TEXT_INACTIVE)
        };

        button::Style {
            background: Some(iced::Background::Color(bg)),
            text_color,
            border: iced::border::Border {
                color: Colors::TRANSPARENT,
                width: 0.0,
                radius: (6.0).into(),
            },
            ..Default::default()
        }
    }
}
