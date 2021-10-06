use std::fs::File;
use std::io::Write;
use std::iter::FromIterator;

use gif::{ColorOutput, DecodeOptions};
use rgb::RGB;

use crate::{clustering, gif_processor};

#[test]
fn sample_gifs()
{
    let input = File::open("test.gif").unwrap();
    let mut out_image = File::create("result.gif").unwrap();
    let out = gif_processor::caption(
        &String::new(),
        input,
        &String::from("my descent into madness is complete"),
    );
    out_image.write(&out).unwrap();
}

#[test]
fn wu_algo()
{
    let input = File::open("test.gif").unwrap();
    let mut options = DecodeOptions::new();
    options.set_color_output(ColorOutput::RGBA);
    let mut decoder = options.read_info(input).unwrap();
    let mut all_colours = Vec::new();
    if let Some(p) = &decoder.global_palette() {
        all_colours.extend(pallette_to_rgb(&p));
    }
    for i in 0..10 {
        if let Some(frame) = decoder.read_next_frame().unwrap() {
            if let Some(p) = &frame.palette {
                println!("{}",i);
                all_colours.extend(pallette_to_rgb(&p));
                all_colours.dedup();
            }
        }
    }
    println!("ok");
    println!("{}", all_colours.len());
    let colours = clustering::compress(&all_colours);
    println!("{}", colours.len());
}

fn pallette_to_rgb(palette: &[u8]) -> Vec<RGB<u8>>
{
    palette
        .chunks(3)
        .map(|x| RGB::from_iter(x.to_owned()))
        .collect()
}
