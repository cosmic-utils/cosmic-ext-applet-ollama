mod localize;
mod request;
mod window;

use window::Window;

pub fn run() -> cosmic::iced::Result {
    localize::localize();

    cosmic::applet::run::<Window>(false, ())
}
