use ashpd::desktop::file_chooser::{FileFilter, SelectedFiles};
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
    theme,
    widget::{self, settings},
    Application, Command, Element,
};

use crate::{
    chat::{
        load_conversation, read_conversation_files, Conversation, Image, ImageAttachment,
        MessageContent, Text,
    },
    fl,
    models::installed_models,
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
    RemoveModel,
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
    ModelPullInput(String),
    BotEvent(stream::Event),
    ToggleContext,
    StopBot,
    SaveConversation,
    SelectedConversation(usize),
    LoadConversation,
    RemoveConversation,
    PullModel,
    ModelsDelSelector(usize),
    DelModel,
    OpenImages,
    ImagesResult(Vec<Image>),
}

pub struct Window {
    core: Core,
    popup: Option<Id>,
    page: Pages,
    prompt: String,
    conversation: Conversation,
    bot_response: String,
    system_messages: Vec<String>,
    models: Vec<String>,
    selected_model: String,
    model_index: Option<usize>,
    last_id: usize,
    chat_id: id::Id,
    keep_context: bool,
    context: Option<Vec<u64>>,
    images: Vec<String>,
    saved_conversations: Vec<String>,
    selected_saved_conv: Option<usize>,
    request: StreamingRequest,
    model_to_pull: String,
    del_model_index: Option<usize>,
    delete_this_model: String,
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
    ) -> (Self, Command<cosmic::app::Message<Self::Message>>) {
        let system_messages = Vec::new();
        let models: Vec<String> = installed_models();

        (
            Self {
                core,
                popup: None,
                page: Pages::Chat,
                prompt: String::new(),
                conversation: Conversation::new(),
                bot_response: String::new(),
                system_messages,
                models: models.clone(),
                selected_model: models[0].clone(),
                model_index: Some(0),
                last_id: 0,
                chat_id: id::Id::new("chat"),
                keep_context: true,
                context: None,
                images: Vec::new(),
                saved_conversations: Vec::new(),
                selected_saved_conv: Some(0),
                request: StreamingRequest::AskWithContext,
                model_to_pull: String::new(),
                del_model_index: Some(0),
                delete_this_model: models[0].clone(),
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
                self.conversation
                    .push(Text::User(MessageContent::Text(self.prompt.clone())));
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
                                self.images.clone(),
                            )))
                        }
                        StreamingRequest::AskWithContext => {
                            _ = tx.blocking_send(stream::Request::AskWithContext((
                                self.selected_model.clone(),
                                self.prompt.clone(),
                                self.images.clone(),
                                self.context.clone(),
                            )))
                        }
                        StreamingRequest::PullModel => {
                            _ = tx.blocking_send(stream::Request::PullModel(
                                self.model_to_pull.clone(),
                            ))
                        }
                        StreamingRequest::RemoveModel => {
                            _ = tx.blocking_send(stream::Request::RemoveModel(
                                self.delete_this_model.clone(),
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
                    self.conversation
                        .push(Text::Bot(MessageContent::Text(self.bot_response.clone())));
                    self.bot_response.clear();
                    self.images.clear();
                }
                stream::Event::PullResponse(status) => {
                    self.status_area_status = status.status;
                }
                stream::Event::PullDone => {
                    self.status_area_status.clear();
                    self.models = installed_models();
                }
                stream::Event::RemovedModel => {
                    self.status_area_status.clear();
                    self.models = installed_models();
                }
                stream::Event::RemoveStatus(status) => {
                    self.status_area_status = status;
                }
            },
            Message::ChangeModel(index) => {
                self.model_index = Some(index);
                self.selected_model = self.models[index].clone();
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
            Message::RemoveConversation => {
                let _ = self
                    .conversation
                    .remove(self.saved_conversations[self.selected_saved_conv.unwrap()].clone());

                self.saved_conversations = read_conversation_files().unwrap();
            }
            Message::PullModel => {
                self.last_id += 1;
                self.request = StreamingRequest::PullModel;
            }
            Message::ModelsDelSelector(index) => {
                self.del_model_index = Some(index);
                self.delete_this_model = self.models[index].clone();
            }
            Message::DelModel => {
                    self.last_id += 1;
                    self.request = StreamingRequest::RemoveModel;
            }
            Message::OpenImages => {
                return Command::perform(
                    async move {
                        let result = SelectedFiles::open_file()
                            .title("Open multiple images")
                            .accept_label("Attach")
                            .modal(true)
                            .multiple(true)
                            .filter(FileFilter::new("JPEG Image").glob("*.jpg"))
                            .filter(FileFilter::new("PNG Image").glob("*.png"))
                            .send()
                            .await
                            .unwrap()
                            .response();

                        if let Ok(result) = result {
                            result
                                .uris()
                                .iter()
                                .map(|file| Image::new(file.path()))
                                .collect::<Vec<Image>>()
                        } else {
                            Vec::new()
                        }
                    },
                    |files| cosmic::app::message::app(Message::ImagesResult(files)),
                );
            }
            Message::ImagesResult(result) => {
                for image in result {
                    self.images.push(image.base64.clone());
                    self.conversation.push(Text::User(MessageContent::Image(
                        ImageAttachment::Raster(image),
                    )));
                }
            }
            Message::ModelPullInput(model) => self.model_to_pull = model
        };

        Command::none()
    }

    fn view(&self) -> Element<Self::Message> {
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

        let open_images = widget::button(
            widget::Container::new(widget::icon::from_name("insert-image-symbolic"))
                .width(32)
                .height(32)
                .center_x()
                .center_y(),
        )
        .on_press(Message::OpenImages)
        .width(42)
        .height(42);

        let clear_chat = widget::button(
            widget::Container::new(widget::icon::from_name("edit-clear-symbolic"))
                .width(32)
                .height(32)
                .center_x()
                .center_y(),
        )
        .on_press(Message::ClearChat)
        .width(42)
        .height(42);

        let stop_bot = widget::button(
            widget::Container::new(widget::icon::from_name("media-playback-stop-symbolic"))
                .width(32)
                .height(32)
                .center_x()
                .center_y(),
        )
        .on_press(Message::StopBot)
        .width(42)
        .height(42);

        let fields = widget::row()
            .push(prompt_input)
            .push(open_images)
            .push(clear_chat)
            .push(stop_bot)
            .spacing(10);

        widget::column()
            .push(padded_control(
                widget::Container::new(Scrollable::new(chat).id(self.chat_id.clone()))
                    .height(Length::Fill),
            ))
            .push(padded_control(fields))
            .height(Length::Fill)
            .into()
    }

    fn settings_view(&self) -> Element<Message> {
        let conv_section = settings::view_section(fl!("conversations"))
            .add(settings::item(
                fl!("keep-context"),
                widget::toggler(None, self.keep_context, |_| Message::ToggleContext),
            ))
            .add(settings::item(
                fl!("select-conversation"),
                widget::dropdown(
                    &self.saved_conversations,
                    self.selected_saved_conv,
                    Message::SelectedConversation,
                ),
            ))
            .add(settings::item(
                fl!("load-conversation"),
                widget::button::standard(fl!("load")).on_press(Message::LoadConversation),
            ))
            .add(settings::item(
                fl!("save-conversation"),
                widget::button::standard(fl!("save")).on_press(Message::SaveConversation),
            ))
            .add(settings::item(
                fl!("remove-conversation"),
                widget::button::standard(fl!("remove")).on_press(Message::RemoveConversation),
            ));

        let models_section = settings::view_section(fl!("manage-models"))
            .add(settings::item_row(vec![
                widget::text_input("llama3:70b", &self.model_to_pull)
                    .width(Length::Fill)
                    .on_input(Message::ModelPullInput)
                    .into(),
                widget::button::standard(fl!("pull-model"))
                    .on_press(Message::PullModel)
                    .into(),
            ]))
            .add(settings::item_row(vec![
                widget::dropdown(
                    &self.models,
                    self.del_model_index,
                    Message::ModelsDelSelector,
                )
                .width(Length::Fill)
                .into(),
                widget::button::standard(fl!("remove-model"))
                    .on_press(Message::DelModel)
                    .into(),
            ]));

        let spacer = widget::Space::with_height(Length::Fill);

        let status_area = widget::row()
            .push(widget::text::monotext("Status: "))
            .push(widget::text::monotext(self.status_area_status.clone()))
            .spacing(10);

        let mut content = widget::column()
            .push(conv_section)
            .push(models_section)
            .push(spacer)
            .spacing(20);

        if !self.status_area_status.is_empty() {
            content = content.push(status_area);
        }

        widget::Container::new(padded_control(content))
            .height(Length::Fill)
            .into()
    }

    fn bot_bubble(&self, message: String) -> Element<Message> {
        let text = widget::text(message);

        let ai = widget::Container::new(text)
            .padding(12)
            .style(theme::Container::List);

        let content = widget::column().push(ai);

        widget::Container::new(content).width(Length::Fill).into()
    }

    fn user_bubble(
        &self,
        message: Option<String>,
        image: Option<widget::image::Handle>,
    ) -> Element<Message> {
        let mut column = widget::column();

        if let Some(mess) = message {
            column = column.push(
                widget::Container::new(widget::text(mess))
                    .padding(12)
                    .style(theme::Container::List),
            )
        }

        if let Some(img) = image {
            column = column.push(
                widget::Container::new(widget::image(img))
                    .padding(12)
                    .style(theme::Container::List),
            )
        }

        let user = widget::Container::new(column)
            .width(Length::Fill)
            .align_x(Horizontal::Right);

        let content = widget::column().push(user);

        widget::Container::new(content).width(Length::Fill).into()
    }

    fn chat_messages(&self, conv: &Conversation) -> Element<Message> {
        let mut content = widget::column().spacing(20);

        for c in &conv.messages {
            match c {
                Text::User(text) => match text {
                    MessageContent::Text(txt) => {
                        if !txt.is_empty() {
                            content = content.push(self.user_bubble(Some(txt.clone()), None))
                        }
                    }
                    MessageContent::Image(image) => match image {
                        ImageAttachment::Svg(_) => todo!(),
                        ImageAttachment::Raster(raster) => {
                            let handle = widget::image::Handle::from_memory(raster.data.clone());

                            content = content.push(self.user_bubble(None, Some(handle)));
                        }
                    },
                },
                Text::Bot(text) => match text {
                    MessageContent::Text(txt) => {
                        if !txt.is_empty() {
                            content = content.push(self.bot_bubble(txt.clone()))
                        }
                    }
                    MessageContent::Image(_) => todo!(),
                },
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
        settings::view_section("")
            .add(settings::item_row(vec![
                widget::button::standard(fl!("chat"))
                    .on_press(Message::ChatPage)
                    .into(),
                widget::button::standard(fl!("settings"))
                    .width(100)
                    .on_press(Message::SettingsPage)
                    .into(),
                widget::dropdown(&self.models, self.model_index, Message::ChangeModel)
                    .width(Length::Fill)
                    .into(),
            ]))
            .into()
    }
}
