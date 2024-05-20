use std::sync::Arc;

use cosmic::{
    app::{message::app, Core, Message as CosmicMessage},
    applet::padded_control,
    iced::{
        self,
        alignment::Horizontal,
        wayland::popup::{destroy_popup, get_popup},
        window::Id,
        Alignment, Length,
    },
    iced_widget::Scrollable,
    theme, widget, Application, Command, Element,
};

use crate::request::{prompt_req, Api, GenerateResponse};

const ID: &'static str = "io.github.elevenhsoft.CosmicAppletOllama";

#[derive(Debug, Clone)]
pub enum Message {
    PopupClosed(Id),
    TogglePopup,
    EnterPrompt(String),
    SendPrompt,
    ReceivedMessage(GenerateResponse),
}

pub struct Window {
    core: Core,
    popup: Option<Id>,
    prompt: Arc<String>,
    user_messages: Vec<String>,
    ollama_responses: Vec<String>,
}

impl Application for Window {
    type Executor = cosmic::SingleThreadExecutor;

    type Flags = ();

    type Message = Message;

    const APP_ID: &'static str = ID;

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(
        core: Core,
        _flags: Self::Flags,
    ) -> (
        Self,
        cosmic::iced::Command<cosmic::app::Message<Self::Message>>,
    ) {
        let user_messages = Vec::new();
        let ollama_responses = Vec::new();

        (
            Self {
                core,
                popup: None,
                prompt: Arc::new(String::new()),
                user_messages,
                ollama_responses,
            },
            Command::none(),
        )
    }

    fn on_close_requested(&self, id: Id) -> Option<Message> {
        Some(Message::PopupClosed(id))
    }

    fn update(&mut self, message: Message) -> Command<CosmicMessage<Message>> {
        match message {
            Message::PopupClosed(id) => {
                if self.popup.as_ref() == Some(&id) {
                    self.popup = None;
                }
            }
            Message::TogglePopup => {
                return if let Some(p) = self.popup.take() {
                    destroy_popup(p)
                } else {
                    let new_id = Id::unique();
                    self.popup.replace(new_id);
                    let mut popup_settings =
                        self.core
                            .applet
                            .get_popup_settings(Id::MAIN, new_id, None, None, None);
                    popup_settings.positioner.size_limits = iced::Limits::NONE
                        .max_width(400.0)
                        .min_width(300.0)
                        .min_height(200.0)
                        .max_height(800.0);
                    get_popup(popup_settings)
                }
            }
            Message::EnterPrompt(prompt) => self.prompt = Arc::new(prompt),
            Message::SendPrompt => {
                let prompt = Arc::clone(&self.prompt);
                self.user_messages.push(self.prompt.to_string());
                self.prompt = Arc::new(String::new());

                return Command::perform(
                    async move {
                        let mut api = Api::new();
                        api.set_model(String::from("llama3"));
                        prompt_req(api, prompt).await
                    },
                    |response| app(Message::ReceivedMessage(response)),
                );
            }
            Message::ReceivedMessage(response) => self.ollama_responses.push(response.response),
        };

        Command::none()
    }

    fn view(&self) -> cosmic::prelude::Element<Self::Message> {
        self.core
            .applet
            .icon_button_from_handle(
                widget::icon::from_name("io.github.elevenhsoft.CosmicAppletOllama").into(),
            )
            .on_press(Message::TogglePopup)
            .into()
    }

    fn view_window(&self, _id: Id) -> Element<Self::Message> {
        let prompt_input = cosmic::iced::widget::text_input("Type something..", &self.prompt)
            .on_input(Message::EnterPrompt)
            .on_submit(Message::SendPrompt)
            .width(Length::Fill);

        let mut chat = widget::Column::new().spacing(10).width(Length::Fill);

        for message in self
            .ollama_responses
            .clone()
            .into_iter()
            .zip(self.user_messages.clone())
        {
            chat = chat.push(self.chat_response(message))
        }

        let content_list = widget::column()
            .push(padded_control(prompt_input))
            .push(padded_control(Scrollable::new(chat)))
            .padding([8, 0]);

        self.core.applet.popup_container(content_list).into()
    }
}

impl Window {
    fn chat_response(&self, message: (String, String)) -> Element<Message> {
        let user = widget::Container::new(
            widget::Container::new(widget::text(message.1))
                .padding(12)
                .style(theme::Container::List),
        )
        .width(Length::Fill)
        .align_x(Horizontal::Right);

        let ai = widget::Container::new(widget::text(message.0))
            .padding(12)
            .style(theme::Container::List);

        let user_col = widget::column().push(user);

        let ai_col = widget::column().push(ai);

        let content = widget::column().push(user_col).push(ai_col).spacing(10);

        widget::Container::new(content).width(Length::Fill).into()
    }
}
