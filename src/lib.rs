pub mod parser;
pub mod renderer;
pub mod sitemap;

mod stylesheet;
mod util;

use std::{
    fs::{copy, create_dir, create_dir_all, remove_dir_all, write, File},
    io::{self},
    path::PathBuf,
};

use log::info;
use parser::parse_error::ParseError;
use renderer::{HtmlLink, HtmlRenderOptions, HtmlRenderer, Meta, Rel};
use sitemap::SiteMap;

use crate::{parser::Parser, sitemap::Node, stylesheet::Stylesheet, util::filestem_from_path};

#[derive(Debug)]
pub enum LssgError {
    ParseError(ParseError),
    Regex(regex::Error),
    Render(String),
    Io(io::Error),
}
impl LssgError {
    pub fn render(error: &str) -> LssgError {
        LssgError::Render(error.into())
    }
}
impl From<ParseError> for LssgError {
    fn from(error: ParseError) -> Self {
        Self::ParseError(error)
    }
}
impl From<io::Error> for LssgError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}
impl From<regex::Error> for LssgError {
    fn from(error: regex::Error) -> Self {
        Self::Regex(error)
    }
}

#[derive(Debug)]
pub struct Link {
    pub rel: Rel,
    pub path: PathBuf,
}

#[derive(Debug)]
pub struct LssgOptions {
    pub index: PathBuf,
    pub output_directory: PathBuf,
    /// Path to a markdown file that describes your 404 page, this will create a seperate html page
    /// which you can you as your not found page.
    pub not_found_page: Option<PathBuf>,
    pub global_stylesheet: Option<PathBuf>,
    pub favicon: Option<PathBuf>,
    /// Overwrite the default stylesheet with your own
    pub overwrite_default_stylesheet: bool,
    /// Add extra resources
    pub links: Vec<Link>,
    pub title: String,
    /// Translates to meta tags <https://www.w3schools.com/tags/tag_meta.asp>
    pub keywords: Vec<(String, String)>,
    /// Lang attribute ("en") <https://www.w3schools.com/tags/ref_language_codes.asp>
    pub language: String,
}

pub struct Lssg {
    options: LssgOptions,
}

impl Lssg {
    pub fn new(options: LssgOptions) -> Lssg {
        Lssg { options }
    }

    // pub fn preview(&self, port: u32) {
    //     info!("Listing on 0.0.0.0:{port}");
    //     todo!()
    // }

    pub fn render(&self) -> Result<(), LssgError> {
        let mut stylesheet = if let Some(p) = &self.options.global_stylesheet {
            let mut s = if self.options.overwrite_default_stylesheet {
                Stylesheet::new()
            } else {
                Stylesheet::default()
            };
            s.append(p)?;
            s
        } else {
            Stylesheet::default()
        };
        for l in self.options.links.iter() {
            if let Rel::Stylesheet = l.rel {
                stylesheet.append(&l.path)?;
            }
        }

        let mut site_map = SiteMap::from_index(self.options.index.clone())?;
        let stylesheet_id =
            site_map.add_stylesheet("main.css".into(), stylesheet, site_map.root())?;

        let favicon = if let Some(input) = &self.options.favicon {
            Some(site_map.add(
                Node {
                    name: "favicon.ico".into(),
                    parent: Some(site_map.root()),
                    children: vec![],
                    node_type: sitemap::NodeType::Resource {
                        input: input.clone(),
                    },
                },
                site_map.root(),
            )?)
        } else {
            None
        };

        if let Some(input) = &self.options.not_found_page {
            let file = File::open(&input)?;
            let _ = site_map.add(
                Node {
                    name: filestem_from_path(input)?,
                    parent: Some(site_map.root()),
                    children: vec![],
                    node_type: sitemap::NodeType::Page {
                        tokens: Parser::parse(file)?,
                        input: input.to_path_buf(),
                        keep_name: true,
                    },
                },
                site_map.root(),
            );
        }

        info!("SiteMap:\n{site_map}");

        let render_options = HtmlRenderOptions {
            links: vec![],
            title: self.options.title.clone(),
            favicon,
            meta: self
                .options
                .keywords
                .iter()
                .map(|(name, content)| Meta {
                    name: name.clone(),
                    content: content.clone(),
                })
                .collect(),
            language: self.options.language.clone(),
        };

        if self.options.output_directory.exists() {
            info!("Removing {:?}", self.options.output_directory);
            remove_dir_all(&self.options.output_directory)?;
        }
        info!("Creating {:?}", self.options.output_directory);
        create_dir_all(&self.options.output_directory)?;

        let mut queue: Vec<usize> = vec![site_map.root()];
        let renderer = HtmlRenderer::new(&site_map);
        while let Some(id) = queue.pop() {
            let node = site_map.get(id)?;
            queue.append(&mut node.children.clone());
            let path = self.options.output_directory.join(site_map.path(id));
            match &node.node_type {
                sitemap::NodeType::Stylesheet(s) => {
                    info!("Writing concatinated stylesheet {path:?}",);
                    write(path, s.to_string())?;
                }
                sitemap::NodeType::Resource { input } => {
                    copy(input, path)?;
                }
                sitemap::NodeType::Folder => {
                    create_dir(path)?;
                }
                sitemap::NodeType::Page { keep_name, .. } => {
                    let mut options = render_options.clone();
                    options.links.push(HtmlLink {
                        rel: renderer::Rel::Stylesheet,
                        href: site_map.rel_path(id, stylesheet_id),
                    });
                    let html = renderer.render(id, options)?;
                    let html_output_path = if *keep_name {
                        path.join(format!("../{}.html", node.name))
                    } else {
                        create_dir_all(&path)?;
                        path.join("index.html")
                    };
                    info!("Writing to {:?}", html_output_path);
                    write(html_output_path, html)?;
                }
            }
        }

        Ok(())
    }
}
