use std::borrow::Cow;
use eframe::egui::{Button, CentralPanel, Color32, CtxRef, FontDefinitions, FontFamily, Hyperlink, Label, Layout, menu, ScrollArea, Separator, TextStyle, TopBottomPanel, Ui};
use eframe::epi::{App, Frame, Storage};

const PADDING: f32 = 5.;
const WHITE: Color32 = Color32::from_rgb(255, 255, 255);
const CYAN: Color32 = Color32::from_rgb(0, 255, 255);

struct NewsCardData
{
    title: String,
    desc: String,
    url: String
}

pub struct Headlines
{
    articles: Vec<NewsCardData>
}

impl Headlines
{
    pub fn new() -> Self
    {
        let iter = (0..20).map(
            |a| NewsCardData
            {
                title: format!("title{}", a),
                desc: format!("desc{}", a),
                url: format!("https://example.com/{}", a)
            }
        );

        Self
        {
            articles: Vec::from_iter(iter)
        }
    }

    fn configure_fonts(&self, ctx: &CtxRef)
    {
        let mut font_def = FontDefinitions::default();

        font_def.font_data.insert
        (
            "MesloLGS".to_string(),
            Cow::Borrowed(include_bytes!("../../MesloLGS-NF-Regular.ttf"))
        );

        font_def.family_and_size.insert
        (
            TextStyle::Heading,
            (FontFamily::Proportional, 35.)
        );

        font_def.family_and_size.insert
        (
            TextStyle::Body,
            (FontFamily::Proportional, 20.)
        );

        font_def.fonts_for_family
            .get_mut(&FontFamily::Proportional)
            .unwrap()
            .insert(0, "MesloLGS".to_string());

        ctx.set_fonts(font_def);
    }

    fn render_news_cards(&self, ui: &mut Ui)
    {
        for a in &self.articles
        {
            // title
            ui.add_space(PADDING);
            let title = format!("▶ {}", a.title);
            ui.colored_label(WHITE, title);

            // desc
            ui.add_space(PADDING);
            let desc = Label::new(&a.desc).text_style(TextStyle::Button);
            ui.add(desc);

            // links
            ui.style_mut().visuals.hyperlink_color = CYAN;
            ui.add_space(PADDING);
            ui.with_layout(Layout::right_to_left(), |ui| {
                ui.add(Hyperlink::new(&a.url).text("read more ⤴"));
            });

            ui.add_space(PADDING);
            ui.add(Separator::default());
        }
    }

    fn render_top_panel(&self, ctx: &CtxRef)
    {
        TopBottomPanel::top("top_panel").show(ctx,
            |ui|
            {
                ui.add_space(10.);
                menu::bar(ui,
                    |ui|
                    {
                        // logo
                        ui.with_layout(Layout::left_to_right(),
                            |ui| { ui.add(Label::new("📓").text_style(TextStyle::Heading)); }
                        );

                        // controls
                        ui.with_layout(Layout::right_to_left(),
                            |ui|
                            {
                                let close_btn = ui.add(Button::new("❌").text_style(TextStyle::Body));
                                let refresh_btn = ui.add(Button::new("🔄").text_style(TextStyle::Body));
                                let theme_btn = ui.add(Button::new("🌙").text_style(TextStyle::Body));
                            }
                        );
                    }
                );
                ui.add_space(10.);
            }
        );
    }
}

impl App for Headlines
{
    fn update(&mut self, ctx: &CtxRef, _frame: &mut Frame<'_>) {
        self.render_top_panel(ctx);
        CentralPanel::default().show(ctx,
            |ui|
            {
                render_header(ui);
                ScrollArea::auto_sized().show(ui,
                    |ui| { self.render_news_cards(ui); }
                );
                render_footer(ctx);
            }
        );
    }

    fn setup(&mut self, ctx: &CtxRef, _frame: &mut Frame<'_>, _storage: Option<&dyn Storage>)
    {
        self.configure_fonts(ctx);
    }

    fn name(&self) -> &str
    {
        "Headlines"
    }
}

fn render_header(ui: &mut Ui)
{
    ui.vertical_centered
    (
        |ui|
        {
            ui.heading("headlines");
        }
    );
    ui.add_space(PADDING);
    let sep = Separator::default().spacing(20.);
    ui.add(sep);
}

fn render_footer(ctx: &CtxRef)
{
    TopBottomPanel::bottom("footer").show(ctx,
        |ui|
        {
            ui.vertical_centered(
                |ui|
                {
                    ui.add_space(10.);

                    // api
                    ui.add(Label::new("API source: newsapi.org").monospace());

                    // egui
                    ui.add(
                        Hyperlink::new("https://github.com/emilk/egui")
                            .text("Made with egui")
                            .text_style(TextStyle::Monospace)
                    );

                    // github repo
                    ui.add(
                        Hyperlink::new("https://github.com/celestomm/headlines")
                            .text("celestomm/headlines")
                            .text_style(TextStyle::Monospace)
                    );

                    ui.add_space(10.);
                }
            );
        }
    );
}