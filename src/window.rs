use ashpd::desktop::file_chooser::{FileFilter, SelectedFiles};
use cosmic::{
    app::{Core, Message as CosmicMessage},
    applet::padded_control,
    iced::{
        self,
        alignment::Horizontal,
        id,
        platform_specific::shell::wayland::commands::popup::{destroy_popup, get_popup},
        theme::Palette,
        window::Id,
        Length, Subscription,
    },
    iced_widget::{
        scrollable::{snap_to, RelativeOffset},
        Scrollable,
    },
    theme,
    widget::{self, settings},
    Application, Element, Task as Command,
};
use std::path::PathBuf;

use crate::{
    chat::{
        load_conversation, read_conversation_files, Conversation, Image, ImageAttachment,
        MessageContent, Text,
    },
    fl,
    models::installed_models,
    stream, Settings,
};

const ID: &str = "io.github.elevenhsoft.CosmicExtAppletOllama";

#[derive(Debug, Clone)]
pub enum Pages {
    Chat,
    Settings,
}

#[derive(Debug, Clone)]
pub enum StreamingRequest {
    Idle,
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
    FindAvatar,
    AvatarResult(PathBuf),
    OllamaAdressFlag(bool),
    OllamaAddressInput(String),
    OllamaAddressSend,
    OpenLink(iced::widget::markdown::Url),
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
    user_avatar: widget::image::Handle,
    ollama_address: String,
    ollama_address_edit: bool,
    settings: Settings,
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
        let settings = Settings::load();
        let model_index = models
            .clone()
            .into_iter()
            .position(|model| model == settings.model);
        let delete_this_model = if !models.is_empty() {
            models[0].clone()
        } else {
            String::new()
        };

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
                selected_model: settings.model.clone(),
                model_index,
                last_id: 0,
                chat_id: id::Id::new("chat"),
                keep_context: settings.keep_context,
                context: None,
                images: Vec::new(),
                saved_conversations: Vec::new(),
                selected_saved_conv: Some(0),
                request: StreamingRequest::Idle,
                model_to_pull: String::new(),
                del_model_index: Some(0),
                delete_this_model,
                status_area_status: String::new(),
                user_avatar: settings.get_avatar_handle(),
                ollama_address: settings.ollama_address.clone(),
                ollama_address_edit: false,
                settings,
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
        let mut commands: Vec<Command<CosmicMessage<Message>>> = Vec::new();

        match message {
            Message::ChatPage => {
                self.page = Pages::Chat;
                commands.push(snap_to(self.chat_id.clone(), RelativeOffset::END));
            }
            Message::SettingsPage => {
                self.page = Pages::Settings;

                self.saved_conversations = read_conversation_files().unwrap();
            }
            Message::PopupClosed(id) => {
                if Some(id) == self.popup {
                    self.popup = None;
                }
            }
            Message::TogglePopup => {
                commands.push(if let Some(p) = self.popup.take() {
                    destroy_popup(p)
                } else {
                    let new_id = Id::unique();
                    self.popup.replace(new_id);
                    let mut popup_settings = self.core.applet.get_popup_settings(
                        self.core.main_window_id().unwrap(),
                        new_id,
                        None,
                        None,
                        None,
                    );
                    popup_settings.positioner.size_limits =
                        iced::Limits::NONE.width(680.0).height(800.0);
                    return get_popup(popup_settings);
                });
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
                        StreamingRequest::Idle => {}
                    }
                    self.prompt.clear();
                }
                stream::Event::Response(message) => {
                    self.bot_response.push_str(&message.response);
                    self.context = message.context;

                    commands.push(snap_to(self.chat_id.clone(), RelativeOffset::END));
                }
                stream::Event::Done => {
                    self.conversation
                        .push(Text::Bot(MessageContent::Text(self.bot_response.clone())));
                    self.bot_response.clear();
                    self.images.clear();
                    self.request = StreamingRequest::Idle;
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
                self.selected_model.clone_from(&self.models[index]);
                self.settings.set_model(self.selected_model.clone());
                let _ = self.settings.save();
            }
            Message::ClearChat => {
                self.prompt.clear();
                self.system_messages.clear();
                self.conversation = Conversation::new();
            }
            Message::ToggleContext => {
                self.keep_context = !self.keep_context;
                self.settings.change_context(self.keep_context);
                let _ = self.settings.save();
            }
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
                self.delete_this_model.clone_from(&self.models[index]);
            }
            Message::DelModel => {
                self.last_id += 1;
                self.request = StreamingRequest::RemoveModel;
            }
            Message::OpenImages => {
                commands.push(Command::perform(
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
                ));
            }
            Message::ImagesResult(result) => {
                for image in result {
                    self.images.push(image.base64.clone());
                    self.conversation.push(Text::User(MessageContent::Image(
                        ImageAttachment::Raster(image),
                    )));
                }
            }
            Message::ModelPullInput(model) => self.model_to_pull = model,
            Message::FindAvatar => commands.push(Command::perform(
                async move {
                    let result = SelectedFiles::open_file()
                        .title("Open image")
                        .accept_label("Attach")
                        .modal(true)
                        .multiple(false)
                        .filter(FileFilter::new("JPEG Image").glob("*.jpg"))
                        .filter(FileFilter::new("PNG Image").glob("*.png"))
                        .send()
                        .await
                        .unwrap()
                        .response();

                    if let Ok(result) = result {
                        let path = result.uris().first().unwrap();
                        if let Ok(path) = path.to_file_path() {
                            path
                        } else {
                            PathBuf::new()
                        }
                    } else {
                        PathBuf::new()
                    }
                },
                |path| cosmic::app::message::app(Message::AvatarResult(path)),
            )),
            Message::AvatarResult(path) => {
                let handle = widget::image::Handle::from_path(&path);
                self.user_avatar = handle;
                self.settings.set_avatar(path);
                let _ = self.settings.save();
            }
            Message::OllamaAdressFlag(flag) => self.ollama_address_edit = flag,
            Message::OllamaAddressInput(input) => self.ollama_address = input,
            Message::OllamaAddressSend => {
                self.settings
                    .set_ollama_address(self.ollama_address.clone());
                let _ = self.settings.save();
            }
            Message::OpenLink(url) => {
                let _ = open::that_in_background(url.to_string());
            }
        };

        Command::batch(commands)
    }

    fn view(&self) -> Element<Self::Message> {
        self.core
            .applet
            .icon_button("io.github.elevenhsoft.CosmicExtAppletOllama-symbolic")
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
            .limits(iced::Limits::NONE.max_width(680.0).max_height(800.0))
            .into()
    }
}

impl Window {
    fn chat_view(&self) -> Element<Message> {
        let mut chat = widget::column().spacing(10).width(Length::Fill);

        chat = chat.push(self.chat_messages(&self.conversation));

        if !self.bot_response.is_empty() {
            chat = chat.push(self.bot_bubble(self.bot_response.clone()));
        }

        for message in &self.system_messages {
            chat = chat.push(self.system_bubble(message.to_string()))
        }

        let prompt_input = widget::text_input(fl!("prompt-field"), &self.prompt)
            .on_input(Message::EnterPrompt)
            .on_submit(Message::SendPrompt)
            .width(Length::Fill);

        let open_images = widget::button::icon(widget::icon::from_name("mail-attachment-symbolic"))
            .on_press(Message::OpenImages);

        let clear_chat = widget::button::icon(widget::icon::from_name("edit-clear-symbolic"))
            .on_press(Message::ClearChat);

        let stop_bot =
            widget::button::icon(widget::icon::from_name("media-playback-stop-symbolic"))
                .on_press(Message::StopBot);

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

    //noinspection ALL
    fn settings_view(&self) -> Element<Message> {
        let conv_section = settings::section::section()
            .title(fl!("conversations"))
            .add(settings::item(
                fl!("user-avatar"),
                widget::button::standard(fl!("open")).on_press(Message::FindAvatar),
            ))
            .add(settings::item(
                fl!("keep-context"),
                widget::toggler(self.keep_context).on_toggle(|_| Message::ToggleContext),
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

        let models_section = settings::section()
            .title(fl!("manage-models"))
            .add(settings::item_row(vec![
                widget::text_input("llava:latest", &self.model_to_pull)
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
            ]))
            .add(settings::item_row(vec![widget::editable_input(
                "IP:PORT",
                &self.ollama_address,
                self.ollama_address_edit,
                Message::OllamaAdressFlag,
            )
            .on_input(Message::OllamaAddressInput)
            .on_submit(Message::OllamaAddressSend)
            .into()]));

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
        let content: Vec<iced::widget::markdown::Item> =
            widget::markdown::parse(&message).collect();
        let markdown = iced::widget::markdown(
            &content,
            iced::widget::markdown::Settings::default(),
            iced::widget::markdown::Style::from_palette(Palette::DARK),
        )
        .map(Message::OpenLink);

        let ai = widget::Container::new(markdown)
            .padding(12)
            .class(theme::Container::List);

        let avatar: &[u8] = include_bytes!("../data/icons/avatar.png");
        let handle = widget::image::Handle::from_bytes(avatar);
        let avatar_widget = widget::image(handle)
            .width(Length::Fixed(48.0))
            .height(Length::Fixed(48.0));

        let message_row = widget::row().push(avatar_widget).push(ai).spacing(12);
        let content = widget::column().push(message_row);

        widget::Container::new(content).into()
    }

    fn user_bubble(&self, message: &MessageContent) -> Element<Message> {
        let mut column = widget::column();

        match message {
            MessageContent::Image(image) => match image {
                ImageAttachment::Svg(_) => todo!(),
                ImageAttachment::Raster(raster) => {
                    let handle = widget::image::Handle::from_bytes(raster.data.clone());

                    column = column.push(widget::image(handle))
                }
            },
            MessageContent::Text(txt) => {
                if !txt.is_empty() {
                    let content: Vec<iced::widget::markdown::Item> =
                        iced::widget::markdown::parse(txt).collect();
                    let markdown = iced::widget::markdown(
                        &content,
                        iced::widget::markdown::Settings::default(),
                        iced::widget::markdown::Style::from_palette(Palette::DARK),
                    )
                    .map(Message::OpenLink);
                    column = column.push(markdown)
                }
            }
        };

        let margin = widget::row().width(50).height(50);

        let container = widget::container(column)
            .padding(12)
            .class(theme::Container::List);

        let avatar_widget = widget::image(self.user_avatar.clone())
            .width(Length::Fixed(48.0))
            .height(Length::Fixed(48.0));

        let message_row = widget::row()
            .push(margin)
            .push(container)
            .push(avatar_widget)
            .spacing(12);

        widget::Container::new(message_row)
            .width(Length::Fill)
            .align_x(Horizontal::Right)
            .into()
    }

    fn chat_messages(&self, conv: &Conversation) -> Element<Message> {
        let mut content = widget::column().spacing(20);

        for c in &conv.messages {
            match c {
                Text::User(message) => content = content.push(self.user_bubble(message)),
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
                .class(theme::Container::List),
        )
        .width(Length::Fill)
        .align_x(Horizontal::Right);

        let user_col = widget::column().push(user);

        let content = widget::column().push(user_col).spacing(10);

        widget::Container::new(content).width(Length::Fill).into()
    }

    fn menu_bar(&self) -> Element<Message> {
        settings::section()
            .title("")
            .add(settings::item_row(vec![
                widget::button::standard(fl!("chat"))
                    .on_press(Message::ChatPage)
                    .into(),
                widget::button::standard(fl!("settings"))
                    .on_press(Message::SettingsPage)
                    .into(),
                widget::dropdown(&self.models, self.model_index, Message::ChangeModel)
                    .width(Length::Fill)
                    .into(),
            ]))
            .into()
    }
}
