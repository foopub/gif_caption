use js_sys::Uint8Array;
use wasm_bindgen::JsValue;
use yew::services::reader::{File, FileData, ReaderService, ReaderTask};
use yew::web_sys::{Blob, Url, BlobPropertyBag};
use yew::{html, ChangeData, Component, ComponentLink, Html, InputData, ShouldRender};

pub fn caption(name: &String, bytes: &Vec<u8>, caption: &String) -> Blob {
    let js_uint_arr = Uint8Array::from(bytes.as_slice());
    //js_intarr = Int8Array::copy_from(i8::from_ne_bytes(bytes));
    //let js_bytes = bytes.iter().map(|x| JsValue::from(*x));
    let blob = Blob::from(JsValue::from(js_uint_arr));
    return blob;
}

//mod gif_processor;

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
    filedata: FileData,
    text: String,
    pending: Vec<ReaderTask>, // no way to create default ReaderTask
    result: Vec<Blob>,
    url: String,
}

impl Component for Model {
    type Message = Msg;
    type Properties = ();

    fn create(_props: Self::Properties, link: ComponentLink<Self>) -> Self {
        Self {
            link,
            filedata: FileData {
                name: String::new(),
                content: Vec::new(),
            }, // surely there's a better way???
            text: String::default(),
            pending: Vec::with_capacity(1),
            result: Vec::with_capacity(1),
            url: String::default(),
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
                self.filedata = filedata;
                self.pending.clear();
                true
            }
            Msg::Start => {
                use std::convert::TryInto;
                //gif_processor::caption(&self.filedata.name, &self.filedata.content, &self.text);
                let js_uint_arr = Uint8Array::new_with_length(self.filedata.content.len().try_into().unwrap());
                js_uint_arr.copy_from(self.filedata.content.as_slice());
                //self.url = js_uint_arr.to_string().into();
                let js_val: JsValue = js_uint_arr.into();
                let blob = Blob::new_with_u8_array_sequence_and_options(
                    &js_val,
                    BlobPropertyBag::new().type_("image/gif")).unwrap();
                self.result.push(blob);
                self.link.callback(|_| Msg::Complete).emit(());
                true
            }
            Msg::Complete => {
                let blob = &self.result[0];
                self.url = Url::create_object_url_with_blob(blob).unwrap();
                true
            }
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
                    <label>{ "Upload gif" }</label>
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
                        type="button" value="Submit"
                        onclick=self.link.callback(|_| Msg::Start)/>
                </form>
                <img src={ format!{"{}", self.url} } />
            </div>
        }
    }
}

fn main() {
    yew::start_app::<Model>();
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
