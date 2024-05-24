use cosmic::{
    app::{Core, Message as CosmicMessage},
    applet::padded_control,
    iced::{
        self,
        alignment::Horizontal,
        id,
        wayland::popup::{destroy_popup, get_popup},
        window::Id,
        Length, Subscription,
    },
    iced_widget::{
        scrollable::{snap_to, RelativeOffset},
        Scrollable,
    },
    theme, widget, Application, Command, Element,
};
use enum_iterator::all;

use crate::{
    chat::{load_conversation, read_conversation_files, Conversation, Text},
    fl,
    models::{is_installed, Models},
    stream,
};

const ID: &str = "io.github.elevenhsoft.CosmicAppletOllama";

#[derive(Debug, Clone)]
pub enum Pages {
    Chat,
    Settings,
}

#[derive(Debug, Clone)]
pub enum StreamingRequest {
    Ask,
    AskWithContext,
    PullModel,
}

#[derive(Debug, Clone)]
pub enum Message {
    ChatPage,
    SettingsPage,
    PopupClosed(Id),
    TogglePopup,
    EnterPrompt(String),
    SendPrompt,
    ChangeModel(usize),
    ClearChat,
    BotEvent(stream::Event),
    ToggleContext,
    StopBot,
    SaveConversation,
    SelectedConversation(usize),
    LoadConversation,
    ModelsPullSelector(usize),
    PullModel,
    ModelsDelSelector(usize),
    DelModel,
}

pub struct Window {
    core: Core,
    popup: Option<Id>,
    page: Pages,
    prompt: String,
    conversation: Conversation,
    bot_response: String,
    system_messages: Vec<String>,
    models: Vec<Models>,
    selected_model: Models,
    model_index: Option<usize>,
    last_id: usize,
    chat_id: id::Id,
    keep_context: bool,
    context: Option<Vec<u64>>,
    saved_conversations: Vec<String>,
    selected_saved_conv: Option<usize>,
    request: StreamingRequest,
    models_to_pull: Vec<Models>,
    pull_model_index: Option<usize>,
    pull_this_model: Option<Models>,
    del_model_index: Option<usize>,
    status_area_status: String,
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

        let models_to_pull = all::<Models>().collect::<Vec<_>>();

        (
            Self {
                core,
                popup: None,
                page: Pages::Chat,
                prompt: String::new(),
                conversation: Conversation::new(),
                bot_response: String::new(),
                system_messages,
                models,
                selected_model: Models::Llama3,
                model_index: Some(0),
                last_id: 0,
                chat_id: id::Id::new("chat"),
                keep_context: true,
                context: None,
                saved_conversations: Vec::new(),
                selected_saved_conv: Some(0),
                request: StreamingRequest::AskWithContext,
                models_to_pull,
                pull_model_index: Some(0),
                pull_this_model: None,
                del_model_index: Some(0),
                status_area_status: String::new(),
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
            Message::ChatPage => self.page = Pages::Chat,
            Message::SettingsPage => {
                self.page = Pages::Settings;

                self.saved_conversations = read_conversation_files().unwrap();
            }
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
                self.conversation.push(Text::User(self.prompt.clone()));
                self.last_id += 1;

                if !self.keep_context {
                    self.request = StreamingRequest::Ask
                } else {
                    self.request = StreamingRequest::AskWithContext
                }
            }
            Message::BotEvent(ev) => match ev {
                stream::Event::Ready(tx) => {
                    match self.request {
                        StreamingRequest::Ask => {
                            _ = tx.blocking_send(stream::Request::Ask((
                                self.selected_model.clone(),
                                self.prompt.clone(),
                            )))
                        }
                        StreamingRequest::AskWithContext => {
                            _ = tx.blocking_send(stream::Request::AskWithContext((
                                self.selected_model.clone(),
                                self.prompt.clone(),
                                self.context.clone(),
                            )))
                        }
                        StreamingRequest::PullModel => {
                            _ = tx.blocking_send(stream::Request::PullModel(
                                self.pull_this_model.as_ref().unwrap().clone(),
                            ))
                        }
                    }
                    self.prompt.clear();
                }
                stream::Event::Response(message) => {
                    self.bot_response.push_str(&message.response);
                    self.context = message.context;

                    return snap_to(self.chat_id.clone(), RelativeOffset::END);
                }
                stream::Event::Done => {
                    self.conversation.push(Text::Bot(self.bot_response.clone()));
                    self.bot_response.clear();
                    self.status_area_status.clear();
                }
                stream::Event::PullResponse(status) => {
                    self.status_area_status = status.status;
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
                self.conversation = Conversation::new();
            }
            Message::ToggleContext => self.keep_context = !self.keep_context,
            Message::StopBot => self.last_id += 1,
            Message::SaveConversation => {
                let _ = self.conversation.save_to_file();
            }
            Message::SelectedConversation(index) => {
                self.selected_saved_conv = Some(index);
            }
            Message::LoadConversation => {
                self.conversation = load_conversation(
                    self.saved_conversations[self.selected_saved_conv.unwrap()].clone(),
                );
            }
            Message::ModelsPullSelector(index) => {
                self.pull_model_index = Some(index);
                self.pull_this_model = Some(self.models_to_pull[index].clone());
            }
            Message::PullModel => {
                if self.pull_this_model.is_some() {
                    self.last_id += 1;
                    self.request = StreamingRequest::PullModel;
                }
            }
            Message::ModelsDelSelector(index) => self.del_model_index = Some(index),
            Message::DelModel => println!("del model"),
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
        let menu_row = widget::row().push(padded_control(self.menu_bar()));

        let page_view = match self.page {
            Pages::Chat => self.chat_view(),
            Pages::Settings => self.settings_view(),
        };

        let content_list = widget::column()
            .push(padded_control(menu_row))
            .push(padded_control(page_view))
            .padding(10);

        self.core
            .applet
            .popup_container(content_list)
            .height(Length::Fill)
            .into()
    }
}

impl Window {
    fn chat_view(&self) -> Element<Message> {
        let mut chat = widget::column().spacing(10).width(Length::Fill);

        chat = chat.push(self.chat_messages(&self.conversation));

        chat = chat.push(self.bot_bubble(if self.bot_response.is_empty() {
            String::from("...")
        } else {
            self.bot_response.clone()
        }));

        for message in &self.system_messages {
            chat = chat.push(self.system_bubble(message.to_string()))
        }

        let prompt_input = widget::text_input(fl!("prompt-field"), &self.prompt)
            .on_input(Message::EnterPrompt)
            .on_submit(Message::SendPrompt)
            .width(Length::Fill);

        let clear_chat =
            widget::button(widget::text(fl!("clear-chat"))).on_press(Message::ClearChat);

        let stop_bot = widget::button(widget::text("Stop"))
            .on_press(Message::StopBot)
            .style(theme::Button::Destructive);

        let fields = widget::row()
            .push(prompt_input)
            .push(clear_chat)
            .push(stop_bot)
            .spacing(10);

        widget::column()
            .push(padded_control(
                widget::Container::new(Scrollable::new(chat).id(self.chat_id.clone())).height(620),
            ))
            .push(padded_control(fields))
            .height(Length::Fill)
            .into()
    }

    fn settings_view(&self) -> Element<Message> {
        let context_title = widget::text::title4("Context");

        let context_toggle = widget::toggler(fl!("keep-context"), self.keep_context, |_| {
            Message::ToggleContext
        });

        let convs_title = widget::text::title4("Manage conversations");

        let convs_dropdown = widget::dropdown(
            &self.saved_conversations,
            self.selected_saved_conv,
            Message::SelectedConversation,
        )
        .width(Length::Fill);

        let load_conv = widget::button(widget::text(fl!("load-conversation")))
            .on_press(Message::LoadConversation);

        let save_conv = widget::button(widget::text(fl!("save-conversation")))
            .on_press(Message::SaveConversation);

        let conv_row = widget::row()
            .push(convs_dropdown)
            .push(load_conv)
            .push(save_conv)
            .spacing(10);

        let spacer = widget::Space::with_height(Length::Fill);

        let status_area = widget::row()
            .push(widget::text::monotext("Status: "))
            .push(widget::text::monotext(self.status_area_status.clone()))
            .spacing(10);

        let mut content = widget::column()
            .push(context_title)
            .push(context_toggle)
            .push(convs_title)
            .push(conv_row)
            .push(self.manage_models())
            .push(spacer)
            .spacing(20);

        if !self.status_area_status.is_empty() {
            content = content.push(status_area);
        }

        widget::Container::new(padded_control(content))
            .height(Length::Fill)
            .into()
    }

    fn manage_models(&self) -> Element<Message> {
        let header = widget::text::title4("Manage models");

        let models_dropdown = widget::dropdown(
            &self.models_to_pull,
            self.pull_model_index,
            Message::ModelsPullSelector,
        )
        .width(Length::FillPortion(3));

        let pull_model = widget::button("Pull model")
            .on_press(Message::PullModel)
            .width(Length::FillPortion(1));

        let del_models_dropdown = widget::dropdown(
            &self.models,
            self.del_model_index,
            Message::ModelsDelSelector,
        )
        .width(Length::FillPortion(3));

        let del_model = widget::button("Remove model")
            .on_press(Message::DelModel)
            .width(Length::FillPortion(1));

        let content = widget::column()
            .push(header)
            .push(
                widget::row()
                    .push(models_dropdown)
                    .push(pull_model)
                    .spacing(10),
            )
            .push(
                widget::row()
                    .push(del_models_dropdown)
                    .push(del_model)
                    .spacing(10),
            )
            .spacing(20);

        widget::Container::new(content).into()
    }

    fn bot_bubble(&self, message: String) -> Element<Message> {
        let text = widget::text(message);

        let ai = widget::Container::new(text)
            .padding(12)
            .style(theme::Container::List);

        let content = widget::column().push(ai);

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

        let content = widget::column().push(user);

        widget::Container::new(content).width(Length::Fill).into()
    }

    fn chat_messages(&self, conv: &Conversation) -> Element<Message> {
        let mut content = widget::column().spacing(20);

        for c in &conv.messages {
            match c {
                Text::User(text) => {
                    if !text.is_empty() {
                        content = content.push(self.user_bubble(text.clone()))
                    }
                }
                Text::Bot(text) => {
                    if !text.is_empty() {
                        content = content.push(self.bot_bubble(text.clone()))
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

    fn menu_bar(&self) -> Element<Message> {
        widget::row()
            .push(
                widget::button(widget::text(fl!("chat")))
                    .width(100)
                    .on_press(Message::ChatPage)
                    .style(theme::Button::Suggested),
            )
            .push(
                widget::button(widget::text(fl!("settings")))
                    .width(100)
                    .on_press(Message::SettingsPage),
            )
            .push(
                widget::dropdown(&self.models, self.model_index, Message::ChangeModel)
                    .width(Length::Fill),
            )
            .spacing(10)
            .into()
    }
}
