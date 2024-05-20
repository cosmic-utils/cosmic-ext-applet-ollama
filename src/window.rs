use cosmic::{
    app::{message::app, Core, Message as CosmicMessage},
    applet::padded_control,
    iced::{
        self,
        alignment::Horizontal,
        wayland::popup::{destroy_popup, get_popup},
        window::Id,
        Length,
    },
    iced_widget::Scrollable,
    theme, widget, Application, Command, Element,
};
use enum_iterator::all;
use std::sync::Arc;

use crate::{
    fl,
    models::{is_installed, Models},
    request::{prompt_req, Api, GenerateResponse},
};

const ID: &'static str = "io.github.elevenhsoft.CosmicAppletOllama";

#[derive(Debug, Clone)]
pub enum Message {
    PopupClosed(Id),
    TogglePopup,
    EnterPrompt(String),
    SendPrompt,
    ReceivedMessage(Option<GenerateResponse>),
    ChangeModel(usize),
    ClearChat,
}

pub struct Window {
    core: Core,
    popup: Option<Id>,
    prompt: Arc<String>,
    user_messages: Vec<String>,
    ollama_responses: Vec<String>,
    system_messages: Vec<String>,
    generating: bool,
    models: Vec<Models>,
    selected_model: Arc<Models>,
    model_index: Option<usize>,
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
        let system_messages = Vec::new();

        let mut models: Vec<Models> = Vec::new();

        for model in all::<Models>().collect::<Vec<_>>() {
            if is_installed(&Arc::new(model.clone())) {
                models.push(model);
            }
        }

        (
            Self {
                core,
                popup: None,
                prompt: Arc::new(String::new()),
                user_messages,
                ollama_responses,
                system_messages,
                generating: false,
                models,
                selected_model: Arc::new(Models::Llama3),
                model_index: Some(0),
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
                    popup_settings.positioner.size_limits =
                        iced::Limits::NONE.width(680.0).height(800.0);
                    get_popup(popup_settings)
                }
            }
            Message::EnterPrompt(prompt) => self.prompt = Arc::new(prompt),
            Message::SendPrompt => {
                let prompt = Arc::clone(&self.prompt);
                let model = Arc::clone(&self.selected_model);
                self.generating = true;
                self.user_messages.push(self.prompt.to_string());
                self.prompt = Arc::new(String::new());

                return Command::perform(
                    async move {
                        let mut api = Api::new();
                        api.set_model(model);
                        prompt_req(api, prompt).await
                    },
                    |response| app(Message::ReceivedMessage(response)),
                );
            }
            Message::ReceivedMessage(response) => {
                self.generating = false;
                if response.is_some() {
                    self.ollama_responses.push(response.unwrap().response)
                } else {
                    self.system_messages.push(fl!("no-response"))
                }
            }
            Message::ChangeModel(index) => {
                self.model_index = Some(index);
                self.selected_model = Arc::new(self.models[index].clone());

                if !is_installed(&self.selected_model) {
                    self.system_messages.push(fl!("model-not-installed"));
                }
            }
            Message::ClearChat => {
                self.system_messages.clear();
                self.user_messages.clear();
                self.ollama_responses.clear();
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

        for message in self
            .ollama_responses
            .clone()
            .into_iter()
            .zip(self.user_messages.clone())
        {
            chat = chat.push(self.chat_messages(message))
        }

        for message in &self.system_messages {
            chat = chat.push(self.system_messages(message.to_string()))
        }

        let generating_info = if self.generating {
            widget::Container::new(widget::text(fl!("chat-typing")))
                .padding(12)
                .style(theme::Container::List)
                .center_x()
        } else {
            widget::Container::new(widget::column())
        };

        chat = chat.push(generating_info);

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
    fn chat_messages(&self, message: (String, String)) -> Element<Message> {
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

    fn system_messages(&self, message: String) -> Element<Message> {
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
