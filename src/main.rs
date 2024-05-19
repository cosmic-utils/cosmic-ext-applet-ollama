mod localize;
mod window;

fn main() -> cosmic::iced::Result {
    localize::localize();

    cosmic::applet::run::<window::Window>(false, ())
}
