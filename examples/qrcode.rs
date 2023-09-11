use dioxus::prelude::*;

use dioxus_scanner::Scanner;

fn main() {
    dioxus_logger::init(log::LevelFilter::Info).expect("failed to init logger");
    dioxus_web::launch(crate::app);
}

fn app(cx: Scope) -> Element {
    render! {
        Scanner {
            cb: |res: rxing::RXingResult| {
                log::info!("Scanned {:?}", res.getText());
            },
        }
    }
}
