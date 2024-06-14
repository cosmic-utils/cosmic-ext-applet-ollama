use std::sync::Mutex;

use cosmic::{
    iced::{
        Length::{self},
        Size,
    },
    iced_core::{
        image, layout,
        widget::{tree, Widget},
    },
    Element, Renderer,
};
use cosmic_text::{Attrs, Buffer, Edit, FontSystem, Metrics, SyntaxEditor};
use markdown::{tokenize, Block, ListItem, Span};

use crate::{FONT_SYSTEM, SWASH_CACHE, SYNTAX_SYSTEM};

fn syntax_theme() -> &'static str {
    if cosmic::theme::is_dark() {
        return "COSMIC Dark";
    }

    "COSMIC Light"
}

pub struct Markdown {
    syntax_editor: Mutex<SyntaxEditor<'static, 'static>>,
    font_system: &'static Mutex<FontSystem>,
    metrics: Metrics,
    margin: f32,
}

impl Markdown {
    pub fn new(content: String) -> Self {
        let metrics = Metrics::new(14.0, 20.0);
        let mut font_system = FONT_SYSTEM.get().unwrap().lock().unwrap();
        let mut buffer = Buffer::new(&mut font_system, metrics);
        let syntax_system = SYNTAX_SYSTEM.get().unwrap();

        let (text, lang) = markdown_to_string(&content);

        buffer.borrow_with(&mut font_system).set_text(
            &text,
            Attrs::new(),
            cosmic_text::Shaping::Advanced,
        );

        let mut editor = SyntaxEditor::new(buffer, syntax_system, syntax_theme()).unwrap();
        editor.syntax_by_extension(&lang);

        Self {
            syntax_editor: Mutex::new(editor),
            font_system: FONT_SYSTEM.get().unwrap(),
            metrics,
            margin: 0.0,
        }
    }

    pub fn margin(&mut self, margin: f32) {
        self.margin = margin;
    }
}

pub struct State {
    handle_opt: Mutex<Option<image::Handle>>,
}

impl State {
    /// Creates a new [`State`].
    pub fn new() -> State {
        State {
            handle_opt: Mutex::new(None),
        }
    }
}

impl<Message> Widget<Message, cosmic::Theme, Renderer> for Markdown {
    fn size(&self) -> Size<cosmic::iced::Length> {
        Size {
            width: Length::Shrink,
            height: Length::Shrink,
        }
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::new())
    }

    fn layout(
        &self,
        _tree: &mut cosmic::iced_core::widget::Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let mut font_system = self.font_system.lock().unwrap();
        let max_width = limits.max().width - self.margin;

        let mut editor = self.syntax_editor.lock().unwrap();
        editor.borrow_with(&mut font_system).shape_as_needed(true);

        editor.with_buffer_mut(|buffer| {
            let mut layout_lines = 0;
            let mut width = 0.0;

            buffer.set_size(
                &mut font_system,
                Some(max_width),
                Some(buffer.metrics().line_height),
            );

            buffer.set_wrap(&mut font_system, cosmic_text::Wrap::Word);

            for line in buffer.lines.iter() {
                match line.layout_opt() {
                    Some(layout) => {
                        layout_lines += layout.len();

                        for l in layout.iter() {
                            if layout_lines > 1 {
                                width = max_width;

                                break;
                            }
                            width = l.w;
                        }
                    }
                    None => (),
                }
            }
            let height = layout_lines as f32 * buffer.metrics().line_height;

            buffer.set_metrics_and_size(
                &mut font_system,
                self.metrics,
                Some(max_width),
                Some(height),
            );

            let size = Size::new(width, height);

            layout::Node::new(size)
        })
    }

    fn draw(
        &self,
        tree: &cosmic::iced_core::widget::Tree,
        renderer: &mut Renderer,
        _theme: &cosmic::Theme,
        style: &cosmic::iced_core::renderer::Style,
        layout: cosmic::iced_core::Layout<'_>,
        _cursor: cosmic::iced_core::mouse::Cursor,
        _viewport: &cosmic::iced::Rectangle,
    ) {
        let state = tree.state.downcast_ref::<State>();

        let mut swash_cache = SWASH_CACHE.get().unwrap().lock().unwrap();
        let mut font_system = self.font_system.lock().unwrap();
        let mut editor = self.syntax_editor.lock().unwrap();

        let scale_factor = style.scale_factor as f32;

        let view_w = layout.bounds().width as i32;
        let view_h = layout.bounds().height as i32;

        let calculate_image_scaled = |view: i32| -> (i32, f32) {
            // Get smallest set of physical pixels that fit inside the logical pixels
            let image = ((view as f32) * scale_factor).floor() as i32;
            // Convert that back into logical pixels
            let scaled = (image as f32) / scale_factor;
            (image, scaled)
        };
        let calculate_ideal = |view_start: i32| -> (i32, f32) {
            // Search for a perfect match within 16 pixels
            for i in 0..16 {
                let view = view_start - i;
                let (image, scaled) = calculate_image_scaled(view);
                if view == scaled as i32 {
                    return (image, scaled);
                }
            }
            let (image, scaled) = calculate_image_scaled(view_start);
            (image, scaled)
        };

        let (image_w, _scaled_w) = calculate_ideal(view_w);
        let (image_h, _scaled_h) = calculate_ideal(view_h);

        editor.shape_as_needed(&mut font_system, true);

        let mut pixels_u8 = vec![0; image_w as usize * image_h as usize * 4];

        let pixels = unsafe {
            std::slice::from_raw_parts_mut(pixels_u8.as_mut_ptr() as *mut u32, pixels_u8.len() / 4)
        };

        let mut handle_opt = state.handle_opt.lock().unwrap();

        if editor.redraw() || handle_opt.is_none() {
            editor.with_buffer(|buffer| {
                buffer.draw(
                    &mut font_system,
                    &mut swash_cache,
                    cosmic_text::Color(0xFFFFFF),
                    |x, y, w, h, color| {
                        draw_rect(
                            pixels,
                            Canvas {
                                w: image_w,
                                h: image_h,
                            },
                            Canvas {
                                w: w as i32,
                                h: h as i32,
                            },
                            Offset { x, y },
                            color,
                        );
                    },
                );
            });
        }

        *handle_opt = Some(image::Handle::from_pixels(
            image_w as u32,
            image_h as u32,
            pixels_u8,
        ));

        if let Some(ref handle) = *handle_opt {
            image::Renderer::draw(
                renderer,
                handle.clone(),
                image::FilterMethod::Nearest,
                cosmic::iced::Rectangle {
                    x: layout.position().x,
                    y: layout.position().y,
                    width: image_w as f32,
                    height: image_h as f32,
                },
                [0.0; 4],
            );
        }
    }
}

struct Canvas {
    w: i32,
    h: i32,
}

struct Offset {
    x: i32,
    y: i32,
}

// source: https://github.com/pop-os/cosmic-edit/blob/master/src/text_box.rs#L136-L215
fn draw_rect(
    buffer: &mut [u32],
    canvas: Canvas,
    offset: Canvas,
    screen: Offset,
    cosmic_color: cosmic_text::Color,
) {
    // Grab alpha channel and green channel
    let mut color = cosmic_color.0 & 0xFF00FF00;
    // Shift red channel
    color |= (cosmic_color.0 & 0x00FF0000) >> 16;
    // Shift blue channel
    color |= (cosmic_color.0 & 0x000000FF) << 16;

    let alpha = (color >> 24) & 0xFF;
    match alpha {
        0 => {
            // Do not draw if alpha is zero.
        }
        255 => {
            // Handle overwrite
            for x in screen.x..screen.x + offset.w {
                if x < 0 || x >= canvas.w {
                    // Skip if y out of bounds
                    continue;
                }

                for y in screen.y..screen.y + offset.h {
                    if y < 0 || y >= canvas.h {
                        // Skip if x out of bounds
                        continue;
                    }

                    let line_offset = y as usize * canvas.w as usize;
                    let offset = line_offset + x as usize;
                    buffer[offset] = color;
                }
            }
        }
        _ => {
            let n_alpha = 255 - alpha;
            for y in screen.y..screen.y + offset.h {
                if y < 0 || y >= canvas.h {
                    // Skip if y out of bounds
                    continue;
                }

                let line_offset = y as usize * canvas.w as usize;
                for x in screen.x..screen.x + offset.w {
                    if x < 0 || x >= canvas.w {
                        // Skip if x out of bounds
                        continue;
                    }

                    // Alpha blend with current value
                    let offset = line_offset + x as usize;
                    let current = buffer[offset];
                    if current & 0xFF000000 == 0 {
                        // Overwrite if buffer empty
                        buffer[offset] = color;
                    } else {
                        let rb = ((n_alpha * (current & 0x00FF00FF))
                            + (alpha * (color & 0x00FF00FF)))
                            >> 8;
                        let ag = (n_alpha * ((current & 0xFF00FF00) >> 8))
                            + (alpha * (0x01000000 | ((color & 0x0000FF00) >> 8)));
                        buffer[offset] = (rb & 0x00FF00FF) | (ag & 0xFF00FF00);
                    }
                }
            }
        }
    }
}

pub fn markdown(content: String) -> Markdown {
    Markdown::new(content)
}

impl<'a, Message> From<Markdown> for Element<'a, Message> {
    fn from(value: Markdown) -> Self {
        Self::new(value)
    }
}

pub fn markdown_to_string(content: &str) -> (String, String) {
    let mut result = String::new();
    let mut language = String::new();

    for block in tokenize(content) {
        let (text, lang) = parse_block(block);

        result.push_str(&format!("{}\n", text));

        if language.is_empty() {
            language.push_str(&lang)
        }
    }

    (result, language)
}

fn parse_block(block: Block) -> (String, String) {
    let mut result = String::new();
    let mut language = String::new();

    match block {
        markdown::Block::Header(header, _len) => {
            if !header.is_empty() {
                for span in header {
                    result.push_str(&format!("{}\n", parse_span(span)))
                }
            }
        }
        markdown::Block::Paragraph(para) => {
            if !para.is_empty() {
                for span in para {
                    result.push_str(&parse_span(span))
                }
            }
        }
        markdown::Block::Blockquote(block) => {
            if !block.is_empty() {
                for block in block {
                    let (text, lang) = parse_block(block);

                    result.push_str(&text);

                    if language.is_empty() {
                        language.push_str(language_to_extension(lang));
                    }
                }
            }
        }
        markdown::Block::CodeBlock(lang, code) => {
            if let Some(lang) = lang {
                if language.is_empty() {
                    language.push_str(language_to_extension(lang));
                }
            }
            result.push_str(&format!("\n{}\n", code));
        }
        markdown::Block::OrderedList(listitem, _listtype) => {
            if !listitem.is_empty() {
                for (num, item) in listitem.into_iter().enumerate() {
                    let text = parse_listitem(item);

                    result.push_str(&format!("\n{}. {}", num + 1, text));
                }
            }
        }
        markdown::Block::UnorderedList(listitem) => {
            if !listitem.is_empty() {
                for item in listitem {
                    let text = parse_listitem(item);

                    result.push_str(&format!("\n - {}", text));
                }
            }
        }
        markdown::Block::Raw(raw) => result.push_str(&raw),
        markdown::Block::Hr => result.push('\n'),
    }

    (result, language)
}

fn parse_listitem(listitem: ListItem) -> String {
    let mut result = String::new();

    match listitem {
        ListItem::Simple(simple) => {
            if !simple.is_empty() {
                for span in simple {
                    result.push_str(&format!("  {}", parse_span(span)))
                }
            }
        }
        ListItem::Paragraph(para) => {
            if !para.is_empty() {
                for block in para {
                    let (text, _lang) = parse_block(block);

                    result.push_str(&text);
                }
            }
        }
    }

    result
}

fn parse_span(span: Span) -> String {
    let mut result = String::new();

    match span {
        markdown::Span::Break => result.push('\n'),
        markdown::Span::Text(text) => result.push_str(&text),
        markdown::Span::Code(code) => result.push_str(&code),
        markdown::Span::Link(_, _, _) => {}
        markdown::Span::Image(_, _, _) => {}
        markdown::Span::Emphasis(emphasis) => {
            for span in emphasis {
                result.push_str(&parse_span(span));
            }
        }
        markdown::Span::Strong(strong) => {
            for span in strong {
                result.push_str(&parse_span(span));
            }
        }
    }

    result
}

fn language_to_extension(lang: String) -> &'static str {
    match lang.as_str() {
        "markdown" => "md",
        "rust" => "rs",
        "python" => "py",
        _ => "",
    }
}
