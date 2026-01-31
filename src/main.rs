mod api;
mod app;
mod models;
mod pages;
mod services;
mod theme;
mod ui;
mod utils;

use app::App;



fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .subscription(App::subscription)
        .run()
}
