use gif_processor::ColourCompression;
use gloo_file::Blob;
use yew::services::reader::{File, FileData, ReaderService, ReaderTask};
use yew::services::ConsoleService;
use yew::web_sys::Url;
use yew::{
    html, ChangeData, Component, ComponentLink, Html, InputData, ShouldRender,
};
use yew::virtual_dom::VNode;

mod clustering;
mod gif_processor;

#[cfg(test)]
mod gif_test;

pub enum Msg
{
    File(File),
    Loaded(FileData),
    Opt(Opts),
    Compression,
    Start,
    Complete,
    NoOp,
}

pub enum Opts
{
    Caption(String),
    Scale(f32),
    FontSize(f32),
    //Compression(bool),
    NumberColours(u8),
}

#[derive(Default)]
struct OptStruct
{
    caption: String,
    scale: Option<f32>,
    font_size: Option<f32>,
    number_colours: ColourCompression,
}

pub struct Model
{
    link: ComponentLink<Self>,
    filedata: Option<FileData>,
    opts: OptStruct,
    pending: Option<ReaderTask>, // no way to create default ReaderTask
    result: Option<Blob>, // should bre replaced with Result?
    url: String,
    compression: VNode,
}

impl Component for Model
{
    type Message = Msg;
    type Properties = ();

    fn create(_props: Self::Properties, link: ComponentLink<Self>) -> Self
    {
        Self {
            link,
            filedata: None,
            opts: OptStruct::default(),
            pending: None,// Vec::with_capacity(1),
            result: None,
            url: String::default(),
            compression: html!(),
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender
    {
        match msg {
            Msg::File(file) => {
                //self.pending.clear();
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
                //self.pending.push(task);
                self.pending = Some(task);
                false
            }
            Msg::Opt(opt) => {
                match opt {
                    Opts::Caption(caption) => self.opts.caption = caption,
                    Opts::Scale(scale) => {
                        self.opts.scale = Some((scale - 1.0).clamp(0.1, 3.0))
                    }
                    Opts::FontSize(size) => {
                        self.opts.font_size =
                            if size > 0.0 { Some(size) } else { None }
                    }
                    Opts::NumberColours(num) => {
                        self.opts.number_colours = ColourCompression::Wu(num.clamp(4,255))
                    }
                }
                false
            }
            Msg::Compression => {
                if self.compression.eq(&html!()) {
                    self.compression = html!(
                    <div class="form-div">
                        <label>{ "Number of colours " }</label>
                        <input
                            type="number" placeholder="auto"
                            oninput=self.link.callback(|e: InputData| {
                                Msg::Opt(Opts::NumberColours(e.value.parse()
                                                        .unwrap_or(255)))
                            })
                        />
                    </div>
                    );
                } else {
                    self.compression = html!();
                    self.opts.number_colours = ColourCompression::None;
                }
                true
            }
            Msg::Loaded(filedata) => {
                self.filedata = Some(filedata);
                //self.pending.clear();
                self.pending = None;
                true
            }
            Msg::Start => {
                //self.result.clear();
                let filedata = self.filedata.as_ref().unwrap();
                let processed = gif_processor::caption(
                    &filedata.name,
                    filedata.content.as_slice(),
                    &self.opts.caption,
                    self.opts.number_colours,
                    self.opts.scale,
                    self.opts.font_size,
                );
                let blob = Blob::new_with_options(
                    processed.as_slice(),
                    Some("image/gif"),
                );
                self.result = Some(blob);
                self.link.callback(|_| Msg::Complete).emit(());
                false
            }
            Msg::Complete => {
                ConsoleService::log("Done");
                if let Some(blob) = &self.result {
                    self.url =
                        Url::create_object_url_with_blob(blob.as_ref()).unwrap();
                }
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
                <form>
                <div class="form-div">
                    <label>{ "Upload gif: " }</label>
                    <input
                        type="file"
                        onchange=self.link.callback(move |value| {
                            if let ChangeData::Files(files) = value{
                                if files.length() > 0 {
                                    let result = files.item(0).unwrap();
                                    if result.type_() == "image/gif" {
                                        return Msg::File(result)
                                    } else {
                                        ConsoleService::log("Wrong file type");
                                    }
                                }
                            }
                            return Msg::NoOp
                        })
                    />
                </div>

                <div class="form-div">
                    <label>{ "Caption" }</label>
                    <input
                        type="text" id="caption" name="caption"
                        oninput=self.link.callback(|e: InputData| {
                            Msg::Opt(Opts::Caption(e.value))
                        })
                    />
                </div>

                <div class="form-div">
                    <label>{ "Scale" }</label>
                    <input
                        type="number" step="0.05" min="1.1" max="4"
                        oninput=self.link.callback(|e: InputData| {
                            Msg::Opt(Opts::Scale(e.value.parse().unwrap_or(1.3)))
                        })
                    />
                </div>

                <div class="form-div">
                    <label>{ "Font size" }</label>
                    <input
                        type="number" placeholder="auto"
                        oninput=self.link.callback(|e: InputData| {
                            Msg::Opt(Opts::FontSize(e.value.parse()
                                                    .unwrap_or(-1.0)))
                        })
                    />
                </div>

                <div class="form-div">
                    <label>{ "Colour compression" }</label>
                    <input type="checkbox"
                    onclick=self.link.callback(|_| Msg::Compression)
                    />
                </div>

                { self.compression.clone() }

                <div class="form-div">
                    <input
                        type="button" value="Submit"
                        disabled=self.filedata.is_none()
                        onclick=self.link.callback(|_| Msg::Start)/>
                </div>

                </form>
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
