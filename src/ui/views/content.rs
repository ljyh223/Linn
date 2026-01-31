use crate::app::Message;
use crate::pages::Page;
use crate::theme::Colors;
use iced::widget::{column, container, text};
use iced::{Element, Fill};

pub struct Content {
    current_page: Page,
}

impl Content {
    pub fn new(current_page: Page) -> Self {
        Self { current_page }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let content = self.view_page_content();

        container(content)
            .width(Fill)
            .height(Fill)
            .padding(40)
            .style(|_theme| container::Style {
                background: Some(iced::Background::Color(Colors::MAIN_BG)),
                ..Default::default()
            })
            .into()
    }

    fn view_page_content(&self) -> Element<'_, Message> {
        column![
            text(self.current_page.title()).size(32),
            text(self.current_page.description())
                .size(16)
                .style(|_theme| text::Style {
                    color: Some(Colors::TEXT_DESCRIPTION),
                }),
        ]
        .spacing(10)
        .into()
    }
}
