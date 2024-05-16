#![allow(non_snake_case)]
#![windows_subsystem = "windows"]

use dioxus::desktop::{Config, WindowBuilder};
use dioxus::prelude::*;
use tracing::Level;

use std::io::Cursor;
use std::path::Path;
use anyhow::Result;
use dicom::object::open_file;
use dicom::dictionary_std::tags;
use dicom::pixeldata::PixelDecoder;
use base64::{engine::general_purpose, Engine as _};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Card {
    filePath: String,
    fileName: String,
    imgSrc: String,
    studyDate: String,
    modality: String,
    institutionName: String,
    patientName: String
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AppState {
    isError: bool,
    cards: Option<Vec<Card>>,
}

impl AppState {
    fn new() -> Self {
        Self {
            isError: false,
            cards: None
        }
    }
}

fn main() {
    // Init logger
    dioxus_logger::init(Level::INFO).expect("failed to init logger");
    
    const VERSION: &str = env!("CARGO_PKG_VERSION");
    let title = format!("dicom dripper v{}", VERSION);
    LaunchBuilder::desktop().with_cfg(Config::new().with_window(WindowBuilder::new().with_title(title))).launch(App)
}

#[component]
fn App() -> Element {
    use_context_provider(|| Signal::new(AppState::new()));

    rsx! {
        style { {include_str!("../assets/main.css")} }
        style { {include_str!("../assets/bulma.min.css")} }
        link { rel: "stylesheet", href: "main.css" }
        link { rel: "stylesheet", href: "bulma.min.css" }
        ErrorMsg {}
        InputFiles{}
        Cards {}
    }
}

#[component]
fn InputFiles() -> Element {
    let mut app_state = consume_context::<Signal<AppState>>();

    rsx! {
        div {
            class: "px-3 py-3",
            input {
                r#type: "file",
                accept: ".dcm",
                multiple: true,
                onchange: move |evt| {
                    async move {
                        if let Some(file_engine) = evt.files() {
                            let files = file_engine.files();
                            if !files.is_empty() {
                                *app_state.write() = AppState::new();
                                let mut cards = Vec::new();
                                for file in & files {
                                    tracing::info!("file: {:?}", file);
                                    match to_card(file) {
                                        Ok(card) => {
                                            cards.push(card);
                                        }
                                        Err(e) => {
                                            tracing::error!("error: {:?}", e);
                                            app_state.write().isError = true;
                                        }
                                    }
                                }
                                app_state.write().cards = Some(cards);
                                app_state.write().isError = false;
                            }
                        } else {
                            app_state.write().isError = true;
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn Cards() -> Element {
    let app_state = consume_context::<Signal<AppState>>();
    let cards_state = app_state.read().cards.clone();

    match cards_state {
        Some(cards) => {
            rsx! {
                div {
                    class: "fixed-grid has-3-cols m-6",
                    div {
                        class: "grid",
                        for card in cards {
                            div {
                                class: "cell",
                                div {
                                    class: "card",
                                    header {
                                        class: "card-header",
                                        p {
                                            class: "card-header-title",
                                            "{card.filePath}"
                                        }
                                    }
                                    div {
                                        class: "card-image",
                                        figure {
                                            class: "image is-4by3",
                                            img { src: card.imgSrc.clone() }
                                        }
                                    }
                                    div {
                                        class: "card-content",
                                        div {
                                            class: "media",
                                            div {
                                                class: "media-content",
                                                p {
                                                    class: "title is-4",
                                                    "{card.patientName}"
                                                }
                                                p {
                                                    class: "subtitle is-6",
                                                    "{card.institutionName}"
                                                }
                                            }
                                        }
                                        div {
                                            class: "content",
                                            "Modality: {card.modality}, Study date: {card.studyDate}"
                                        }
                                    }
                                    footer {
                                        class: "card-footer",
                                        a {
                                            href: "{card.imgSrc}",
                                            download: "{card.fileName}.png",
                                            class: "card-footer-item",
                                            "Extract image"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        None => None
    }
}

#[component]
fn ErrorMsg() -> Element {
    let app_state = consume_context::<Signal<AppState>>();
    let is_error_state = app_state.read().isError;

    match is_error_state {
        true => {
            rsx! {
                div {
                    class: "container px-3 py-3",
                    article {
                        class: "message is-danger",
                        div {
                            class: "message-body",
                            "Failed to load file."
                        }
                    }    
                }
            }
        }
        false => None
    }
}

fn to_card(file_path: &String) -> Result<Card>{
    let path = Path::new(file_path);
    let file_name = path.file_stem().unwrap().to_str().unwrap().to_string();
    let obj = open_file(file_path)?;
    let pixel_data = obj.decode_pixel_data()?;
    let img: dicom_pixeldata::image::DynamicImage = pixel_data.to_dynamic_image(0)?;
    let rgb = img.to_luma8();
    let mut bytes: Vec<u8> = Vec::new();
    rgb
    .write_to(&mut Cursor::new(&mut bytes), dicom_pixeldata::image::ImageFormat::Png)
    .expect("Couldn't write image to bytes.");
    let b64 = general_purpose::STANDARD.encode(bytes);
    let data_url = format!("data:image/png;base64,{}", b64);
    let tag_study_date = &obj.element(tags::STUDY_DATE)?.to_str()?.to_string();
    let study_date: String = format!("{}-{}-{}", &tag_study_date[0..4], &tag_study_date[4..6], &tag_study_date[6..8]);
    Ok(Card {
        filePath: file_path.clone(),
        fileName: file_name,
        imgSrc: data_url,
        studyDate: study_date,
        modality: obj.element(tags::MODALITY)?.to_str()?.to_string(),
        institutionName: obj.element(tags::INSTITUTION_NAME)?.to_str()?.to_string(),
        patientName: obj.element(tags::PATIENT_NAME)?.to_str()?.to_string(),
    })
}