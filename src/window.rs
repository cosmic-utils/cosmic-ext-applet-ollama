use cosmic::{
    app::{Core, Message as CosmicMessage},
    applet::padded_control,
    iced::{
        self,
        alignment::Horizontal,
        wayland::popup::{destroy_popup, get_popup},
        window::Id,
        Length, Subscription,
    },
    iced_widget::Scrollable,
    theme, widget, Application, Command, Element,
};
use enum_iterator::all;

use crate::{
    fl,
    models::{is_installed, Models},
    stream,
};

const ID: &'static str = "io.github.elevenhsoft.CosmicAppletOllama";

#[derive(Debug, Clone)]
pub enum Conversation {
    User(String),
    Bot(String),
}

#[derive(Debug, Clone)]
pub enum Message {
    PopupClosed(Id),
    TogglePopup,
    EnterPrompt(String),
    SendPrompt,
    ChangeModel(usize),
    ClearChat,
    BotEvent(stream::Event),
}

pub struct Window {
    core: Core,
    popup: Option<Id>,
    prompt: String,
    conversation: Vec<Conversation>,
    bot_response: String,
    system_messages: Vec<String>,
    models: Vec<Models>,
    selected_model: Models,
    model_index: Option<usize>,
    last_id: usize,
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
        let system_messages = Vec::new();

        let mut models: Vec<Models> = Vec::new();

        for model in all::<Models>().collect::<Vec<_>>() {
            if is_installed(&model.clone()) {
                models.push(model);
            }
        }

        (
            Self {
                core,
                popup: None,
                prompt: String::new(),
                conversation: Vec::new(),
                bot_response: String::new(),
                system_messages,
                models,
                selected_model: Models::Llama3,
                model_index: Some(0),
                last_id: 0,
            },
            Command::none(),
        )
    }

    fn on_close_requested(&self, id: Id) -> Option<Message> {
        Some(Message::PopupClosed(id))
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch(vec![
            stream::subscription(self.last_id).map(Message::BotEvent)
        ])
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
                    popup_settings.positioner.size_limits =
                        iced::Limits::NONE.width(680.0).height(800.0);
                    get_popup(popup_settings)
                }
            }
            Message::EnterPrompt(prompt) => self.prompt = prompt,
            Message::SendPrompt => {
                self.conversation
                    .push(Conversation::User(self.prompt.clone()));
                self.last_id += 1;
            }
            Message::BotEvent(ev) => match ev {
                stream::Event::Ready(tx) => {
                    _ = tx.blocking_send(stream::Request::Ask((
                        self.selected_model.clone(),
                        self.prompt.clone(),
                    )));
                    self.prompt.clear();
                }
                stream::Event::Response(message) => {
                    let response = message.response;
                    self.bot_response.push_str(&response);
                }
                stream::Event::Done => {
                    self.conversation
                        .push(Conversation::Bot(self.bot_response.clone()));
                    self.bot_response.clear();
                }
            },
            Message::ChangeModel(index) => {
                self.model_index = Some(index);
                self.selected_model = self.models[index].clone();

                if !is_installed(&self.selected_model) {
                    self.system_messages.push(fl!("model-not-installed"));
                }
            }
            Message::ClearChat => {
                self.prompt.clear();
                self.system_messages.clear();
                self.conversation.clear();
            }
        };

        Command::none()
    }

    fn view(&self) -> cosmic::prelude::Element<Self::Message> {
        self.core
            .applet
            .icon_button("io.github.elevenhsoft.CosmicAppletOllama-symbolic")
            .on_press(Message::TogglePopup)
            .into()
    }

    fn view_window(&self, _id: Id) -> Element<Self::Message> {
        let prompt_input = cosmic::iced::widget::text_input(&fl!("prompt-field"), &self.prompt)
            .on_input(Message::EnterPrompt)
            .on_submit(Message::SendPrompt)
            .width(Length::Fill);

        let clear_chat = widget::button(widget::text(fl!("clear-chat")))
            .on_press(Message::ClearChat)
            .style(theme::Button::Destructive);

        let models_dropdown =
            widget::dropdown(&self.models, self.model_index, Message::ChangeModel).width(220);

        let fields = widget::row()
            .push(prompt_input)
            .push(models_dropdown)
            .push(clear_chat)
            .spacing(6);

        let mut chat = widget::Column::new().spacing(10).width(Length::Fill);

        chat = chat.push(self.chat_messages(self.conversation.clone()));

        chat = chat.push(self.bot_bubble(if self.bot_response.is_empty() {
            String::from("...")
        } else {
            self.bot_response.clone()
        }));

        for message in &self.system_messages {
            chat = chat.push(self.system_bubble(message.to_string()))
        }

        let content_list = widget::column()
            .push(padded_control(fields))
            .push(padded_control(Scrollable::new(chat).height(Length::Fill)))
            .padding([8, 0]);

        self.core
            .applet
            .popup_container(content_list)
            .height(Length::Fill)
            .into()
    }
}

impl Window {
    fn bot_bubble(&self, message: String) -> Element<Message> {
        let line = widget::row().push(widget::text(message.to_owned()));

        let ai = widget::Container::new(line)
            .padding(12)
            .style(theme::Container::List);

        let ai_col = widget::column().push(ai);
        let content = widget::column().push(ai_col).spacing(10);

        widget::Container::new(content).width(Length::Fill).into()
    }

    fn user_bubble(&self, message: String) -> Element<Message> {
        let user = widget::Container::new(
            widget::Container::new(widget::text(message))
                .padding(12)
                .style(theme::Container::List),
        )
        .width(Length::Fill)
        .align_x(Horizontal::Right);

        let user_col = widget::column().push(user);
        let content = widget::column().push(user_col);

        widget::Container::new(content).width(Length::Fill).into()
    }

    fn chat_messages(&self, conv: Vec<Conversation>) -> Element<Message> {
        let mut content = widget::column().spacing(20);

        for c in conv {
            match c {
                Conversation::User(text) => {
                    if !text.is_empty() {
                        content = content.push(self.user_bubble(text))
                    }
                }
                Conversation::Bot(text) => {
                    if !text.is_empty() {
                        content = content.push(self.bot_bubble(text))
                    }
                }
            }
        }

        widget::Container::new(content).width(Length::Fill).into()
    }

    fn system_bubble(&self, message: String) -> Element<Message> {
        let user = widget::Container::new(
            widget::Container::new(widget::text(message))
                .padding(12)
                .style(theme::Container::List),
        )
        .width(Length::Fill)
        .align_x(Horizontal::Right);

        let user_col = widget::column().push(user);

        let content = widget::column().push(user_col).spacing(10);

        widget::Container::new(content).width(Length::Fill).into()
    }
}
