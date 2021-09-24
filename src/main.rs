#![allow(dead_code)]

use yew::services::reader::{File, FileData, ReaderService, ReaderTask};
use yew::{html, ChangeData, Component, ComponentLink, Html, InputData, ShouldRender};

type FileName = String;
type Chunks = bool;

pub enum Msg {
    File(File),
    Loaded(FileData),
    Text(String),
    Start,
    Complete,
    NoOp,
}

pub struct Model {
    link: ComponentLink<Model>,
    file: FileData,
    text: String,
    pending: Vec<ReaderTask>, // no way to create default ReaderTask
}

impl Component for Model {
    type Message = Msg;
    type Properties = ();

    fn create(_props: Self::Properties, link: ComponentLink<Self>) -> Self {
        Self {
            link,
            file: FileData {
                name: String::new(),
                content: Vec::new(),
            }, // surely there's a better way???
            text: String::default(),
            pending: Vec::with_capacity(1),
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::File(file) => {
                self.pending.clear();
                let task = ReaderService::read_file(
                    file,
                    self.link.callback(move |filedata| Msg::Loaded(filedata)),
                )
                .unwrap();
                self.pending.push(task);
                true
            }
            Msg::Text(caption) => {
                self.text = caption;
                true
            }
            Msg::Loaded(filedata) => {
                self.file = filedata;
                self.pending.clear();
                true
            }
            Msg::Start => true,
            Msg::Complete => true,
            Msg::NoOp => true,
        }
    }

    fn change(&mut self, _props: Self::Properties) -> ShouldRender {
        false
    }

    fn view(&self) -> Html {
        html! {
            <div>
                <form>
                    <p>{ "Upload gif" }</p>
                    <input
                        type="file"
                        onchange=self.link.callback(move |value| {
                            if let ChangeData::Files(files) = value{
                                let result = files.item(0).unwrap();
                                if result.type_() == "image/gif" {
                                    Msg::File(result)
                                } else {
                                    Msg::NoOp //add error message here
                                }
                            } else {
                                Msg::NoOp
                            }
                        })
                    />
                    <label>{ "Caption" }</label>
                    <input
                        type="text" id="caption" name="caption"
                        oninput=self.link.callback(|e: InputData| Msg::Text(e.value))/>
                    <input
                        type="submit" value="Submit"
                        onclick=self.link.callback(|_| Msg::Start)/>
                </form>
            </div>
        }
    }
}

//receive uploaded gif

//get dimensions

//?normalise small/large gifs

//set text size relative to gif size
//set margin size

//calculate splitting

//make sized box and add text
//fn make_box(w: u16, h: u16, text: String) -> () {}

//stick box to every frame image
