use gloo_file::Blob;
use yew::services::reader::{File, FileData, ReaderService, ReaderTask};
use yew::services::ConsoleService;
use yew::web_sys::Url;
use yew::{
    html, ChangeData, Component, ComponentLink, Html, InputData, ShouldRender,
};

mod clustering;
mod gif_processor;

#[cfg(test)]
mod gif_test;

pub enum Msg
{
    File(File),
    Loaded(FileData),
    Text(String),
    Start,
    Complete,
    NoOp,
}

pub struct Model
{
    link: ComponentLink<Model>,
    filedata: FileData,
    text: String,
    pending: Vec<ReaderTask>, // no way to create default ReaderTask
    result: Vec<Blob>,
    url: String,
}

impl Component for Model
{
    type Message = Msg;
    type Properties = ();

    fn create(_props: Self::Properties, link: ComponentLink<Self>) -> Self
    {
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

    fn update(&mut self, msg: Self::Message) -> ShouldRender
    {
        match msg {
            Msg::File(file) => {
                self.pending.clear();
                ConsoleService::log(
                    format!("File size: {}", file.size()).as_str(),
                );
                let blob: Blob = file.clone().into();
                self.url =
                    Url::create_object_url_with_blob(blob.as_ref()).unwrap();
                let task = ReaderService::read_file(
                    file,
                    self.link.callback(Msg::Loaded),
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
                self.result.clear();
                let processed = gif_processor::caption(
                    &self.filedata.name,
                    self.filedata.content.as_slice(),
                    &self.text,
                );
                let blob = Blob::new_with_options(
                    processed.as_slice(),
                    Some("image/gif"),
                );
                self.result.push(blob);
                self.link.callback(|_| Msg::Complete).emit(());
                true
            }
            Msg::Complete => {
                ConsoleService::log("Done");
                let blob = &self.result[0];
                self.url =
                    Url::create_object_url_with_blob(blob.as_ref()).unwrap();
                true
            }
            Msg::NoOp => true,
        }
    }

    fn change(&mut self, _props: Self::Properties) -> ShouldRender
    {
        false
    }

    fn view(&self) -> Html
    {
        html! {
            <div>
            <div>
                <label>{ "Upload gif: " }</label>
                <input
                    type="file"
                    onchange=self.link.callback(move |value| {
                        if let ChangeData::Files(files) = value{
                            //panics if cancel
                            if files.length() != 0 {
                                let result = files.item(0).unwrap();
                                if result.type_() == "image/gif" {
                                    Msg::File(result)
                                } else {
                                    ConsoleService::log("Wrong file type");
                                    Msg::NoOp
                                }
                            } else {
                                // clicked cancel
                                Msg::NoOp
                            }
                        } else  {
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
            </div>
            <div>
            <img src={ self.url.to_string() } />
            </div>
            </div>
        }
    }
}

fn main()
{
    yew::start_app::<Model>();
}
