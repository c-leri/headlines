use std::borrow::Cow;
use std::sync::mpsc::{channel, Receiver, Sender, sync_channel, SyncSender};
#[cfg(not(target_arch = "wasm32"))]
use std::thread;
use eframe::egui::{Button, CentralPanel, Color32, CtxRef, FontDefinitions, FontFamily, Hyperlink, Key, Label, Layout, menu, ScrollArea, Separator, TextStyle, TopBottomPanel, Ui, Visuals, Window};
use eframe::epi::{App, Frame, Storage};
use serde::{Serialize, Deserialize};
use newsapi::NewsAPI;

const PADDING: f32 = 5.;
const WHITE: Color32 = Color32::from_rgb(255, 255, 255);
const BLACK: Color32 = Color32::from_rgb(0,0,0);
const CYAN: Color32 = Color32::from_rgb(0, 255, 255);
const RED: Color32 = Color32::from_rgb(255,0,0);

enum Msg
{
    APIKeySet(String),
    Refresh
}

#[derive(Serialize, Deserialize)]
struct HeadlinesConfig
{
    dark_mode: bool,
    api_key: String
}

impl Default for HeadlinesConfig
{
    fn default() -> Self
    {
        Self
        {
            dark_mode: true,
            api_key: String::new()
        }
    }
}

struct NewsCardData
{
    title: String,
    desc: String,
    url: String
}

pub struct Headlines
{
    articles: Vec<NewsCardData>,
    config: HeadlinesConfig,
    api_key_initialized: bool,
    news_rx: Option<Receiver<NewsCardData>>,
    app_tx: Option<SyncSender<Msg>>
}

impl Headlines
{
    pub fn new() -> Self
    {
        Self
        {
            articles: Vec::new(),
            api_key_initialized: Default::default(),
            config: Default::default(),
            news_rx: None,
            app_tx: None
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
        if self.articles.is_empty()
        {
            ui.vertical_centered
            (
                |ui|
                    {
                        ui.label("Loading ⌛");
                    }
            );
        }
        else {
            for a in &self.articles
            {
                // title
                ui.add_space(PADDING);
                let title = format!("▶ {}", a.title);
                if self.config.dark_mode
                { ui.colored_label(WHITE, title); } else { ui.colored_label(BLACK, title); }

                // desc
                ui.add_space(PADDING);
                let desc = Label::new(&a.desc).text_style(TextStyle::Button);
                ui.add(desc);

                // links
                if self.config.dark_mode
                { ui.style_mut().visuals.hyperlink_color = CYAN; } else { ui.style_mut().visuals.hyperlink_color = RED; }
                ui.add_space(PADDING);
                ui.with_layout(Layout::right_to_left(), |ui| {
                    ui.add(Hyperlink::new(&a.url).text("read more ⤴"));
                });

                ui.add_space(PADDING);
                ui.add(Separator::default());
            }
        }
    }

    fn render_top_panel(&mut self, ctx: &CtxRef, _frame: &mut Frame<'_>)
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
                                #[cfg(not(target_arch = "wasm32"))]
                                let close_btn = ui.add(Button::new("❌").text_style(TextStyle::Body));
                                #[cfg(not(target_arch = "wasm32"))]
                                if close_btn.clicked()
                                {
                                    _frame.quit();
                                }

                                let refresh_btn = ui.add(Button::new("🔄").text_style(TextStyle::Body));
                                if refresh_btn.clicked()
                                {
                                    if let Some(tx) = &self.app_tx
                                    {
                                        self.articles.clear();
                                        if let Err(e) = tx.send(Msg::Refresh)
                                        {
                                            tracing::error!("Failed sending msg: {}", e);
                                        }
                                    }
                                }

                                let theme_btn = ui
                                    .add(Button::new({
                                        if self.config.dark_mode
                                        { "🌞" }
                                        else
                                        { "🌙" }
                                    }).text_style(TextStyle::Body));
                                if theme_btn.clicked()
                                {
                                    self.config.dark_mode = !self.config.dark_mode;
                                }
                            }
                        );
                    }
                );
                ui.add_space(10.);
            }
        );
    }

    fn render_config(&mut self, ctx: &CtxRef)
    {
        CentralPanel::default().show(ctx,
            |_ui|
            {
                Window::new("Configuration").show(ctx,
                    |ui|
                    {
                        ui.label("Enter your API key for newsapi.org");
                        let text_input = ui.text_edit_singleline(&mut self.config.api_key);
                        if text_input.lost_focus() && ui.input().key_pressed(Key::Enter)
                        {
                            self.api_key_initialized = true;
                            if let Some(tx) = &self.app_tx
                            {
                                if let Err(e) = tx.send(Msg::APIKeySet(self.config.api_key.to_string()))
                                {
                                    tracing::error!("Failed sending msg: {}", e)
                                }
                            }
                            tracing::error!("API key set");
                        }
                        ui.label("If you haven't registered for the API key, head over to");
                        ui.hyperlink("https://newsapi.org");
                    }
                );
            }
        );
    }

    fn preload_articles(&mut self)
    {
        if let Some(rx) = &self.news_rx
        {
            match rx.try_recv()
            {
                Ok(news_data) => {
                    self.articles.push(news_data);
                }
                Err(e) => {
                    tracing::warn!("Error receiving msg: {}", e);
                }
            }
        }
    }
}

impl App for Headlines
{
    fn update(&mut self, ctx: &CtxRef, frame: &mut Frame<'_>) {
        ctx.request_repaint();

        if self.config.dark_mode
        {
            ctx.set_visuals(Visuals::dark());
        }
        else
        {
            ctx.set_visuals(Visuals::light());
        }

        if !self.api_key_initialized
        { self.render_config(ctx); }
        else {
            self.preload_articles();

            self.render_top_panel(ctx, frame);
            CentralPanel::default().show(ctx,
                |ui|
                {
                    render_header(ui);
                    ScrollArea::auto_sized().show(ui,
                        |ui|
                        { self.render_news_cards(ui); }
                    );
                    render_footer(ctx);
                }
            );
        }
    }

    fn setup(&mut self, ctx: &CtxRef, _frame: &mut Frame<'_>, storage: Option<&dyn Storage>)
    {
        if let Some(storage) = storage
        {
            self.config = eframe::epi::get_value(storage, "headlines").unwrap_or_default();
            self.api_key_initialized = !self.config.api_key.is_empty();
        }

        let api_key = self.config.api_key.to_string();

        #[cfg(not(target_arch = "wasm32"))]
        let (mut news_tx, news_rx) = channel();
        #[cfg(target_arch = "wasm32")]
        let (news_tx, news_rx) = channel();

        self.news_rx = Some(news_rx);

        let (app_tx, app_rx) = sync_channel(1);
        self.app_tx = Some(app_tx);

        #[cfg(not(target_arch = "wasm32"))]
        thread::spawn(
            move ||
            {
                if !api_key.is_empty()
                { fetch_news(&api_key, &mut news_tx); }
                else
                {
                    loop
                    {
                        match app_rx.recv()
                        {
                            Ok(Msg::APIKeySet(api_key)) => {
                                fetch_news(&api_key, &mut news_tx);
                            }
                            Ok(Msg::Refresh) => {
                                fetch_news(&api_key, &mut news_tx);
                            }
                            Err(e) => {
                                tracing::error!("Failed receiving msg: {}", e);
                            }
                        }
                    }
                }
            }
        );

        #[cfg(target_arch = "wasm32")]
        let api_key_web = api_key.clone();
        #[cfg(target_arch = "wasm32")]
        let news_tx_web = news_tx.clone();

        #[cfg(target_arch = "wasm32")]
        gloo_timers::callback::Timeout::new(10,
            move ||
            {
                wasm_bindgen_futures::spawn_local(
                    async
                    {
                        fetch_web(api_key_web, news_tx_web).await;
                    }
                );
            }
        ).forget();

        #[cfg(target_arch = "wasm32")]
        gloo_timers::callback::Interval::new(500,
            move ||
            {
                match app_rx.try_recv()
                {
                    Ok(Msg::APIKeySet(api_key)) => {
                        wasm_bindgen_futures::spawn_local(fetch_web(api_key.clone(), news_tx.clone()));
                    }
                    Ok(Msg::Refresh) => {
                        wasm_bindgen_futures::spawn_local(fetch_web(api_key.clone(), news_tx.clone()));
                    }
                    Err(e) => {
                        tracing::error!("Failed receiving msg: {}", e);
                    }
                }
            }
        ).forget();

        self.configure_fonts(ctx);
    }

    fn save(&mut self, storage: &mut dyn Storage) {
        eframe::epi::set_value(storage, "headlines", &self.config);
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

#[cfg(not(target_arch = "wasm32"))]
fn fetch_news(api_key: &str, news_tx: &mut Sender<NewsCardData>)
{
    if let Ok(response) = NewsAPI::new(api_key).fetch()
    {
        for article in response.articles()
        {
            let news = NewsCardData
            {
                title: article.title().to_string(),
                desc: article.description().map(|s| s.to_string()).unwrap_or("...".to_string()),
                url: article.url().to_string()
            };
            if let Err(e) = news_tx.send(news)
            {
                tracing::error!("Error sending news data: {}", e);
            }
        }
    }
    else
    {
        tracing::error!("Failed fetching news");
    }
}

#[cfg(target_arch = "wasm32")]
async fn fetch_web(api_key: String, news_tx: Sender<NewsCardData>)
{
    if let Ok(response) = NewsAPI::new(&api_key).fetch_web().await
    {
        for article in response.articles()
        {
            let news = NewsCardData
            {
                title: article.title().to_string(),
                desc: article.description().map(|s| s.to_string()).unwrap_or("...".to_string()),
                url: article.url().to_string()
            };
            if let Err(e) = news_tx.send(news)
            {
                tracing::error!("Error sending news data: {}", e);
            }
        }
    }
    else
    {
        tracing::error!("Failed fetching news");
    }
}