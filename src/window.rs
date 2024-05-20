use std::sync::Arc;

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

use crate::{
    fl,
    models::Models,
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
}

pub struct Window {
    core: Core,
    popup: Option<Id>,
    prompt: Arc<String>,
    user_messages: Vec<String>,
    ollama_responses: Vec<String>,
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

        let models: Vec<Models> = vec![
            Models::NoModel,
            Models::Llama3,
            Models::Llama370b,
            Models::Phi3,
            Models::Mistral,
            Models::NeuralChat,
            Models::Starling,
            Models::CodeLlama,
            Models::Llama2Uncensored,
            Models::LlaVa,
            Models::Gemma,
            Models::Gemma7b,
            Models::Solar,
        ];

        (
            Self {
                core,
                popup: None,
                prompt: Arc::new(String::new()),
                user_messages,
                ollama_responses,
                generating: false,
                models,
                selected_model: Arc::new(Models::NoModel),
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
                    popup_settings.positioner.size_limits = iced::Limits::NONE
                        .max_width(680.0)
                        .min_width(300.0)
                        .min_height(600.0)
                        .max_height(800.0);
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
                }
            }
            Message::ChangeModel(index) => {
                self.model_index = Some(index);
                self.selected_model = Arc::new(self.models[index].clone());
            }
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
        let prompt_input = cosmic::iced::widget::text_input(&fl!("prompt-field"), &self.prompt)
            .on_input(Message::EnterPrompt)
            .on_submit(Message::SendPrompt)
            .width(Length::Fill);

        let models_dropdown =
            widget::dropdown(&self.models, self.model_index, Message::ChangeModel).width(220);

        let fields = widget::row()
            .push(prompt_input)
            .push(models_dropdown)
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
            .push(padded_control(Scrollable::new(chat)))
            .padding([8, 0]);

        self.core.applet.popup_container(content_list).into()
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
}
