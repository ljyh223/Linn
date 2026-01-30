mod app;
mod pages;
mod theme;
mod ui;

use app::App;

fn main() -> iced::Result {
    iced::application(App::default, App::update, App::view).run()
}
