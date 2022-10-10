use eframe::egui::Vec2;
use eframe::{NativeOptions, run_native};
use headlines::Headlines;

fn main() {
    tracing_subscriber::fmt::init();

    let headlines = Headlines::new();
    let mut win_option = NativeOptions::default();
    win_option.initial_window_size = Some(Vec2::new(540., 960.));

    run_native("headlines", win_option, Box::new(|cc| Box::new(headlines.init(cc))));
}
