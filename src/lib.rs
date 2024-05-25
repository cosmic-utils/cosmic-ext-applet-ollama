mod api;
mod chat;
mod localize;
mod models;
mod stream;
mod window;

use window::Window;

pub fn run() -> cosmic::iced::Result {
    localize::localize();

    cosmic::applet::run::<Window>(false, ())
}
