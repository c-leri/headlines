use eframe::egui::{
    menu, Align, Button, CentralPanel, Color32, Context, FontData, FontDefinitions, FontFamily,
    Hyperlink, Key, Label, Layout, RichText, ScrollArea, Separator, TextStyle, TopBottomPanel, Ui,
    Visuals, Window,
};
use eframe::{App, CreationContext, Frame, Storage};
use newsapi::{NewsAPI, NewsAPIResponse, Country};
use serde::{Deserialize, Serialize};
use std::sync::mpsc::{channel, sync_channel, Receiver, Sender, SyncSender};
#[cfg(not(target_arch = "wasm32"))]
use std::thread;

const PADDING: f32 = 5.;
const WHITE: Color32 = Color32::from_rgb(255, 255, 255);
const BLACK: Color32 = Color32::from_rgb(0, 0, 0);
const CYAN: Color32 = Color32::from_rgb(0, 255, 255);
const RED: Color32 = Color32::from_rgb(255, 0, 0);

const APP_NAME: &str = "headlines";

enum Msg {
    APIKeySet(String),
    Refresh(Country),
}

#[derive(Serialize, Deserialize)]
struct HeadlinesConfig {
    dark_mode: bool,
    api_key: String,
    country: Country
}

impl Default for HeadlinesConfig {
    fn default() -> Self {
        Self {
            dark_mode: true,
            api_key: String::new(),
            country: Country::FR
        }
    }
}

struct NewsCardData {
    title: String,
    desc: String,
    url: String,
}

pub struct Headlines {
    articles: Vec<NewsCardData>,
    config: HeadlinesConfig,
    api_key_initialized: bool,
    news_rx: Option<Receiver<NewsCardData>>,
    app_tx: Option<SyncSender<Msg>>,
}

impl Headlines {
    pub fn new() -> Self {
        Self {
            articles: Vec::new(),
            api_key_initialized: Default::default(),
            config: Default::default(),
            news_rx: None,
            app_tx: None,
        }
    }

    fn configure_fonts(&self, ctx: &Context) {
        let mut font_def = FontDefinitions::default();

        font_def.font_data.insert(
            "MesloLGS".to_string(),
            FontData::from_static(include_bytes!("../MesloLGS-NF-Regular.ttf")),
        );

        font_def
            .families
            .get_mut(&FontFamily::Proportional)
            .unwrap()
            .insert(0, "MesloLGS".to_string());

        ctx.set_fonts(font_def);
    }

    fn render_news_cards(&self, ui: &mut Ui) {
        if self.articles.is_empty() {
            ui.vertical_centered(|ui| {
                ui.label("Loading ⌛");
            });
        } else {
            for a in &self.articles {
                // title
                ui.add_space(PADDING);
                let title = format!("▶ {}", a.title);
                if self.config.dark_mode {
                    ui.colored_label(WHITE, title);
                } else {
                    ui.colored_label(BLACK, title);
                }

                // desc
                ui.add_space(PADDING);
                let desc = Label::new(RichText::new(&a.desc).text_style(TextStyle::Button));
                ui.add(desc);

                // links
                if self.config.dark_mode {
                    ui.style_mut().visuals.hyperlink_color = CYAN;
                } else {
                    ui.style_mut().visuals.hyperlink_color = RED;
                }
                ui.add_space(PADDING);
                ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                    ui.add(Hyperlink::from_label_and_url("read more ⤴", &a.url));
                });

                ui.add_space(PADDING);
                ui.add(Separator::default());
            }
        }
    }

    fn render_top_panel(&mut self, ctx: &Context, _frame: &mut Frame) {
        TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.add_space(10.);
            menu::bar(ui, |ui| {
                // logo
                ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
                    ui.add(Label::new(
                        RichText::new("📓").text_style(TextStyle::Heading),
                    ));
                });

                // controls
                ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                    #[cfg(not(target_arch = "wasm32"))]
                    let close_btn =
                        ui.add(Button::new(RichText::new("❌").text_style(TextStyle::Body)));
                    #[cfg(not(target_arch = "wasm32"))]
                    if close_btn.clicked() {
                        _frame.close();
                    }

                    let refresh_btn =
                        ui.add(Button::new(RichText::new("🔄").text_style(TextStyle::Body)));
                    if refresh_btn.clicked() {
                        if let Some(tx) = &self.app_tx {
                            self.articles.clear();
                            tx.send(Msg::Refresh(self.config.country)).expect("Failed sending refresh event");
                        }
                    }

                    let theme_btn = ui.add(Button::new(
                        RichText::new({
                            if self.config.dark_mode {
                                "🌞"
                            } else {
                                "🌙"
                            }
                        })
                        .text_style(TextStyle::Body),
                    ));
                    if theme_btn.clicked() {
                        self.config.dark_mode = !self.config.dark_mode;
                    }

                    let country_btn =
                        ui.add(Button::new(RichText::new("🌐").text_style(TextStyle::Body)));
                    if country_btn.clicked() {
                        let country;
                        match self.config.country {
                            Country::US => { country = Country::FR; }
                            Country::FR => { country = Country::US; }
                        }
                        self.config.country = country;

                        if let Some(tx) = &self.app_tx {
                            self.articles.clear();
                            tx.send(Msg::Refresh(country)).expect("Failed sending refresh event");
                        }
                    }

                    let settings_btn =
                        ui.add(Button::new(RichText::new("🛠").text_style(TextStyle::Body)));
                    if settings_btn.clicked() {
                        self.api_key_initialized = !self.api_key_initialized;
                    }
                });
            });
            ui.add_space(10.);
        });
    }

    fn render_config(&mut self, ctx: &Context) {
        CentralPanel::default().show(ctx, |_| {
            Window::new("Configuration").show(ctx, |ui| {
                ui.label("Enter your API key for newsapi.org");
                let text_input = ui.text_edit_singleline(&mut self.config.api_key);
                if text_input.lost_focus() && ui.input().key_pressed(Key::Enter) {
                    self.api_key_initialized = true;
                    if let Some(tx) = &self.app_tx {
                        tx.send(Msg::APIKeySet(self.config.api_key.to_string()))
                            .expect("Failed sending APIKeySet event");
                    }
                    tracing::info!("API key set");
                }
                ui.label("If you haven't registered for the API key, head over to");
                ui.hyperlink("https://newsapi.org");
            });
        });
    }

    fn preload_articles(&mut self) {
        if let Some(rx) = &self.news_rx {
            match rx.try_recv() {
                Ok(news_data) => {
                    self.articles.push(news_data);
                }
                Err(_) => {}
            }
        }
    }

    pub fn init(mut self, cc: &CreationContext) -> Self {
        if let Some(storage) = cc.storage {
            self.config = eframe::get_value(storage, APP_NAME).unwrap_or_default();
            self.api_key_initialized = !self.config.api_key.is_empty();
            tracing::info!(self.api_key_initialized);
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
        thread::spawn(move || {
            if !api_key.is_empty() {
                fetch_news(&api_key, self.config.country, &mut news_tx);
            }
            loop {
                match app_rx.recv() {
                    Ok(Msg::APIKeySet(api_key)) => {
                        fetch_news(&api_key, self.config.country, &mut news_tx);
                    }
                    Ok(Msg::Refresh(country)) => {
                        fetch_news(&api_key, country, &mut news_tx);
                    }
                    Err(e) => {
                        tracing::error!("Failed receiving msg: {}", e);
                    }
                }
            }
        });

        #[cfg(target_arch = "wasm32")]
        {
            let api_key_web = api_key.clone();
            let news_tx_web = news_tx.clone();
            gloo_timers::callback::Timeout::new(10, move || {
                wasm_bindgen_futures::spawn_local(async move {
                    fetch_web(api_key_web, self.config.country, news_tx_web).await;
                });
            })
            .forget();

            gloo_timers::callback::Interval::new(500, move || match app_rx.try_recv() {
                Ok(Msg::APIKeySet(api_key)) => {
                    wasm_bindgen_futures::spawn_local(fetch_web(api_key.clone(), self.config.country, news_tx.clone()));
                }
                Ok(Msg::Refresh(country)) => {
                    wasm_bindgen_futures::spawn_local(fetch_web(api_key.clone(), country, news_tx.clone()));
                }
                Err(e) => {
                    tracing::error!("Failed receiving msg: {}", e);
                }
            })
            .forget();
        }

        self.configure_fonts(&cc.egui_ctx);

        self
    }
}

impl App for Headlines {
    fn update(&mut self, ctx: &Context, frame: &mut Frame) {
        ctx.request_repaint();

        if self.config.dark_mode {
            ctx.set_visuals(Visuals::dark());
        } else {
            ctx.set_visuals(Visuals::light());
        }

        if !self.api_key_initialized {
            self.render_config(ctx);
        } else {
            self.preload_articles();

            self.render_top_panel(ctx, frame);

            render_footer(ctx);

            CentralPanel::default().show(ctx, |ui| {
                render_header(ui);
                ScrollArea::vertical().show(ui, |ui| {
                    self.render_news_cards(ui);
                });
            });
        }
    }

    fn save(&mut self, storage: &mut dyn Storage) {
        eframe::set_value(storage, "headlines", &self.config);
    }

    fn persist_native_window(&self) -> bool {
        false
    }
}

fn render_header(ui: &mut Ui) {
    ui.vertical_centered(|ui| {
        ui.heading("headlines");
    });
    ui.add_space(PADDING);
    let sep = Separator::default().spacing(20.);
    ui.add(sep);
}

fn render_footer(ctx: &Context) {
    TopBottomPanel::bottom("footer").show(ctx, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(10.);

            // api
            ui.add(Label::new(
                RichText::new("API source: newsapi.org").monospace(),
            ));

            // egui
            ui.add(Hyperlink::from_label_and_url(
                RichText::new("Made with egui").text_style(TextStyle::Monospace),
                "https://github.com/emilk/egui",
            ));

            // github repo
            ui.add(Hyperlink::from_label_and_url(
                RichText::new("celestomm/headlines").text_style(TextStyle::Monospace),
                "https://github.com/celestomm/headlines",
            ));

            ui.add_space(10.);
        });
    });
}

#[cfg(not(target_arch = "wasm32"))]
fn fetch_news(api_key: &str, country: Country, news_tx: &mut Sender<NewsCardData>) {
    if let Ok(response) = NewsAPI::new(api_key).country(country).fetch() {
        generate_news_card_data(&response, news_tx);
    } else {
        tracing::error!("Failed fetching news");
    }
}

#[cfg(target_arch = "wasm32")]
async fn fetch_web(api_key: String, country: Country, news_tx: Sender<NewsCardData>) {
    if let Ok(response) = NewsAPI::new(&api_key).country(country).fetch_web().await {
        generate_news_card_data(&response, &news_tx);
    } else {
        tracing::error!("Failed fetching news");
    }
}

fn generate_news_card_data(response: &NewsAPIResponse, news_tx: &Sender<NewsCardData>) {
    for article in response.articles() {
        let news = NewsCardData {
            title: article.title().to_string(),
            desc: article
                .description()
                .map(|s| s.to_string())
                .unwrap_or("...".to_string()),
            url: article.url().to_string(),
        };
        if let Err(e) = news_tx.send(news) {
            tracing::error!("Error sending news data: {}", e);
        }
    }
}

