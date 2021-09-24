use gif;
//use std::io::Read;
use std::borrow::Cow;
use yew::services::ConsoleService;

const SCALE: f64 = 1.2;

pub fn caption<'a>(name: &String, bytes: &'a Vec<u8>, caption: &String) -> Vec<u8> {
    let mut options = gif::DecodeOptions::new();
    options.set_color_output(gif::ColorOutput::RGBA);
    let mut decoder = options.read_info(bytes.as_slice()).unwrap();
    ConsoleService::log(
        format!("Dimensions: w {}, h {}", decoder.width(), decoder.height()).as_str(),
    );
    let h = decoder.height() as f64;
    let w = decoder.width();
    let new_h: f64 = h * SCALE;
    let new_h = new_h as u16;
    let new_a = vec![0; (new_h * w * 3).into()];

    let mut out_image = Vec::new();
    let mut encoder =
        gif::Encoder::new(&mut out_image, w, new_h, decoder.global_palette().unwrap()).unwrap();
    while let Some(old_frame) = decoder.read_next_frame().unwrap() {
        let mut frame = gif::Frame::default();
        frame.width = w;
        frame.height = new_h;
        let out = process_frame(&new_a, &old_frame.buffer);
        frame.buffer = Cow::Borrowed(&out);

        encoder.write_frame(&frame).unwrap();
    }

    drop(encoder);
    out_image
}

fn process_frame(new: &Vec<u8>, buffer: &Cow<[u8]>) -> Vec<u8> {
    let result = [new.clone(), buffer.to_vec()].concat();
    result
}
