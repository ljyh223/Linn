use crate::pages::Page;
use crate::ui::{Content, Sidebar};
use iced::widget::row;
use iced::{Element, Fill};

#[derive(Debug, Clone)]
pub enum Message {
    Navigate(Page),
}

pub struct App {
    current_page: Page,
    sidebar: Sidebar,
    content: Content,
}

impl Default for App {
    fn default() -> Self {
        let current_page = Page::DailyRecommend;
        Self {
            current_page,
            sidebar: Sidebar::new(current_page),
            content: Content::new(current_page),
        }
    }
}

impl App {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Navigate(page) => {
                self.current_page = page;
                self.sidebar = Sidebar::new(page);
                self.content = Content::new(page);
            }
        }
    }

    pub fn view(&self) -> Element<Message> {
        row![self.sidebar.view(), self.content.view()]
            .width(Fill)
            .height(Fill)
            .into()
    }
}
