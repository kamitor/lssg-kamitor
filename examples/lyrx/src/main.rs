use std::path::PathBuf;

use lssg::{renderer::Rel, Link, Lssg, LssgOptions};
use simple_logger::SimpleLogger;

fn main() {
    SimpleLogger::new().init().unwrap();
    Lssg::new(LssgOptions {
        index: PathBuf::from("./content/home.md"),
        output_directory: PathBuf::from("./build"),
        global_stylesheet: Some(PathBuf::from("./content/main.css")),
        overwrite_default_stylesheet: false,
        links: vec![
            Link {
                rel: Rel::Stylesheet,
                path: PathBuf::from("./content/lib/fontawesome.css"),
            },
            Link {
                rel: Rel::Stylesheet,
                path: PathBuf::from("./content/lib/fa-solid.css"),
            },
            Link {
                rel: Rel::Stylesheet,
                path: PathBuf::from("./content/lib/fa-brands.css"),
            },
        ],
        title: "LyrX".into(),
        language: "en".into(),
        keywords: vec![],
    })
    .render()
    .unwrap();
}
