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
use cosmic_text::{
    Attrs, Buffer, Color, Edit, Editor, Family, FontSystem, Metrics, Shaping, Weight,
};
use markdown::{tokenize, Block, ListItem, Span};
use syntect::{
    easy::HighlightLines,
    highlighting::{Style, ThemeSet},
    parsing::SyntaxSet,
    util::LinesWithEndings,
};

use crate::{FONT_SYSTEM, SWASH_CACHE};

fn syntax_theme() -> &'static str {
    if !cosmic::theme::is_dark() {
        return "base16-ocean.light";
    }

    "base16-ocean.dark"
}

fn buffer_text_color() -> cosmic_text::Color {
    if !cosmic::theme::is_dark() {
        return cosmic_text::Color(0x000000);
    }

    cosmic_text::Color(0xFFFFFF)
}

pub struct Markdown {
    syntax_editor: Mutex<Editor<'static>>,
    font_system: &'static Mutex<FontSystem>,
    margin: f32,
}

impl Markdown {
    pub fn new(content: &str) -> Self {
        let metrics = metrics(14.0);
        let font_system = FONT_SYSTEM.get().unwrap();
        let buffer = Buffer::new_empty(metrics);

        let mut editor = Editor::new(buffer);
        let mut parser = Parser::new();
        let blocks = tokenize(content);

        parser.run(Box::leak(Box::new(blocks)));

        editor.with_buffer_mut(|buffer| {
            set_buffer_text(&mut font_system.lock().unwrap(), &mut parser.spans, buffer)
        });

        Self {
            syntax_editor: Mutex::new(editor),
            font_system,
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
            let mut height = 0.0;

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

                        for l in layout.iter() {
                            if let Some(line_height) = l.line_height_opt {
                                height += line_height;
                            } else {
                                height += buffer.metrics().line_height;
                            }
                        }
                    }
                    None => (),
                }
            }

            buffer.set_size(&mut font_system, Some(max_width), Some(height));

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
                    buffer_text_color(),
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

pub fn markdown(content: &str) -> Markdown {
    Markdown::new(content)
}

impl<'a, Message> From<Markdown> for Element<'a, Message> {
    fn from(value: Markdown) -> Self {
        Self::new(value)
    }
}

fn metrics(font_size: f32) -> Metrics {
    let line_height = (font_size * 1.4).ceil();
    Metrics::new(font_size, line_height)
}

fn set_buffer_text(
    font_system: &mut FontSystem,
    collect_spans: &mut [(&'static str, Attrs)],
    buffer: &mut Buffer,
) {
    let attrs = Attrs::new();
    attrs.family(Family::SansSerif);

    buffer.set_rich_text(
        font_system,
        collect_spans.iter().copied(),
        attrs,
        Shaping::Advanced,
        None,
    )
}

struct Parser<'a, 'b> {
    spans: Vec<(&'a str, Attrs<'b>)>,
}

impl<'a, 'b> Parser<'a, 'b>
where
    'b: 'a,
{
    pub fn new() -> Self {
        Self { spans: Vec::new() }
    }

    pub fn run<'block>(&mut self, block: &'block Vec<Block>)
    where
        'block: 'b,
    {
        for block in block {
            self.parse_block(block, Attrs::new());
            self.spans.push(("\n", Attrs::new()));
        }
    }

    fn parse_block<'block>(&mut self, block: &'block Block, attrs: Attrs<'block>)
    where
        'block: 'b,
    {
        match block {
            Block::Header(span, level) => {
                for item in span {
                    match level {
                        1 => {
                            let attrs = attrs.metrics(metrics(24.0));
                            attrs.weight(Weight::BOLD);
                            self.parse_span(item, attrs);
                        }
                        2 => {
                            let attrs = attrs.metrics(metrics(22.0));
                            attrs.weight(Weight::BOLD);
                            self.parse_span(item, attrs);
                        }
                        3 => {
                            let attrs = attrs.metrics(metrics(20.0));
                            attrs.weight(Weight::BOLD);
                            self.parse_span(item, attrs);
                        }
                        4 => {
                            let attrs = attrs.metrics(metrics(18.0));
                            attrs.weight(Weight::BOLD);
                            self.parse_span(item, attrs);
                        }
                        5 => {
                            let attrs = attrs.metrics(metrics(16.0));
                            attrs.weight(Weight::BOLD);
                            self.parse_span(item, attrs);
                        }
                        6 => {
                            let attrs = attrs.metrics(metrics(14.0));
                            attrs.weight(Weight::BOLD);
                            self.parse_span(item, attrs);
                        }
                        _ => self.parse_span(item, Attrs::new()),
                    }
                }
                self.spans.push(("\n", Attrs::new()));
            }
            Block::Paragraph(span) => {
                for item in span {
                    self.parse_span(item, attrs);
                }
                self.spans.push(("\n", Attrs::new()));
            }
            Block::Blockquote(blockquote) => {
                for item in blockquote {
                    let attrs = attrs.family(Family::Monospace);
                    self.parse_block(item, attrs);
                }
                self.spans.push(("\n", Attrs::new()));
            }
            Block::CodeBlock(lang, code) => {
                let extension = if let Some(lang) = lang {
                    language_to_extension(lang)
                } else {
                    "txt"
                };

                let attrs = attrs.family(Family::Monospace);
                let code_block = highlight_code(code, extension, attrs);

                self.spans.extend(code_block);
                self.spans.push(("\n", attrs));
            }
            Block::OrderedList(listitem, _type) => {
                for item in listitem.iter() {
                    let attrs = attrs.family(Family::Serif);
                    self.spans.push((" - ", attrs));
                    self.parse_listitem(item);
                    self.spans.push(("\n", attrs));
                }
                self.spans.push(("\n", Attrs::new()));
            }
            Block::UnorderedList(listitem) => {
                for item in listitem {
                    let attrs = attrs.family(Family::Serif);
                    self.spans.push((" - ", attrs));
                    self.parse_listitem(item);
                    self.spans.push(("\n", attrs));
                }
                self.spans.push(("\n", Attrs::new()));
            }
            Block::Raw(raw_text) => {
                self.spans.push((raw_text, Attrs::new()));
                self.spans.push(("\n", Attrs::new()));
            }
            Block::Hr => self.spans.push(("\n", Attrs::new())),
        }
    }

    fn parse_span<'c>(&mut self, span: &'c Span, attrs: Attrs<'c>)
    where
        'c: 'b,
    {
        match span {
            Span::Break => self.spans.push(("\n", attrs)),
            Span::Text(text) => self.spans.push((text, attrs)),
            Span::Code(code) => {
                let attrs = attrs.family(Family::Monospace);
                self.spans.push((code, attrs));
            }
            Span::Link(_, _, _) => {}
            Span::Image(_, _, _) => {}
            Span::Emphasis(emphasis) => {
                for item in emphasis {
                    let attrs = attrs.family(Family::Cursive);
                    self.parse_span(item, attrs);
                }
            }
            Span::Strong(strong) => {
                for item in strong {
                    let attrs = attrs.weight(Weight::BOLD);
                    self.parse_span(item, attrs);
                }
            }
        }
    }

    fn parse_listitem<'d>(&mut self, item: &'d ListItem)
    where
        'd: 'b,
    {
        match item {
            ListItem::Simple(simple) => {
                for item in simple {
                    self.parse_span(item, Attrs::new());
                }
            }
            ListItem::Paragraph(block) => {
                for item in block {
                    self.parse_block(item, Attrs::new());
                }
            }
        }
    }
}

fn highlight_code<'a>(
    code: &'a str,
    extension: &str,
    attrs: Attrs<'a>,
) -> Vec<(&'a str, Attrs<'a>)> {
    let mut result: Vec<(&'a str, Attrs)> = Vec::new();

    let ps = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();

    let syntax = ps.find_syntax_by_extension(extension).unwrap();
    let mut h = HighlightLines::new(syntax, &ts.themes[syntax_theme()]);
    for line in LinesWithEndings::from(code) {
        let ranges: Vec<(Style, &str)> = h.highlight_line(line, &ps).unwrap();

        for (style, text) in ranges {
            let fg = style.foreground;
            let color = Color::rgb(fg.r, fg.g, fg.b);

            let attrs = attrs.color(color);
            result.push((text, attrs));
        }
    }

    result
}

fn language_to_extension(lang: &str) -> &'static str {
    match lang {
        "python" => "py",
        "javascript" => "js",
        "java" => "java",
        "c" => "c",
        "cpp" => "cpp",
        "c++" => "cpp",
        "csharp" => "cs",
        "c#" => "cs",
        "php" => "php",
        "ruby" => "rb",
        "swift" => "swift",
        "kotlin" => "kt",
        "go" => "go",
        "r" => "r",
        "perl" => "pl",
        "shell" => "sh",
        "bash" => "sh",
        "objective-c" => "m",
        "objective-c++" => "mm",
        "typescript" => "ts",
        "html" => "html",
        "css" => "css",
        "sql" => "sql",
        "matlab" => "m",
        "scala" => "scala",
        "rust" => "rs",
        "dart" => "dart",
        "elixir" => "ex",
        "haskell" => "hs",
        "lua" => "lua",
        "assembly" => "asm",
        "fortran" => "f90",
        "pascal" => "pas",
        "cobol" => "cob",
        "erlang" => "erl",
        "fsharp" => "fs",
        "f#" => "fs",
        "julia" => "jl",
        "groovy" => "groovy",
        "ada" => "adb",
        "markdown" => "md",
        _ => "txt",
    }
}
