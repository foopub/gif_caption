use std::fs::File;
use std::io::Write;
use std::iter::FromIterator;

use gif::{ColorOutput, DecodeOptions, Encoder, Repeat};
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
    let mut decoder = {
        let input = File::open("test.gif").unwrap();
        let mut options = DecodeOptions::new();
        options.set_color_output(ColorOutput::RGBA);
        options.read_info(input).unwrap()
    };

    let mut all_colours = Vec::new();

    if let Some(p) = &decoder.global_palette() {
        all_colours.extend(pallette_to_rgb(&p));
    }
    while let Some(frame) = decoder.read_next_frame().unwrap() {
        if let Some(p) = &frame.palette {
            all_colours.extend(pallette_to_rgb(&p));
        }
    }

    let (colours_flat, indices) = clustering::compress(&all_colours);
    all_colours.dedup();
    //let mut sorted = pallette_to_rgb(&colours_flat);
    //sorted.sort_unstable();
    //for palettes of 256 or less colours, these should be the same!
    //println!("All colours:\n{:?}", all_colours);
    //println!("Reduced palette:\n{:?}", sorted);
    //println!("All colours:\n{:?}", all_colours.len());
    //println!("Reduced palette:\n{:?}", colours_flat.len());
    drop(all_colours);

    let mut out_image = File::create("xresult.gif").unwrap();

    let mut encoder = Encoder::new(
        &mut out_image,
        decoder.width(),
        decoder.height(),
        &colours_flat,
    )
    .unwrap();
    encoder.set_repeat(Repeat::Infinite).unwrap();

    //need to repeat this
    let mut decoder = {
        let input = File::open("test.gif").unwrap();
        let mut options = DecodeOptions::new();
        options.set_color_output(ColorOutput::RGBA);
        options.read_info(input).unwrap()
    };

    let round = |x| x as usize >> 3;

    while let Some(old_frame) = decoder.read_next_frame().unwrap() {
        let mut new_frame = old_frame.clone();

        new_frame.palette = None;

        let triplets: Vec<[u8; 3]> = old_frame
            .buffer
            .chunks_exact(4)
            .map(|x| [x[0], x[1], x[2]])
            .collect();

        let mut new_buff = Vec::with_capacity(triplets.len());

        triplets.iter().for_each(|x| {
            //let i = indices[round(x[0])][round(x[1])][round(x[2])];
            //println!("{}, {:?}, {:?}", i, x, sorted[i as usize]);
            new_buff.push(indices[round(x[0])][round(x[1])][round(x[2])]);
        });
        //println!("{}", new_buff.len());

        new_frame.buffer = new_buff.into();
        encoder.write_frame(&new_frame).unwrap();
    }
    drop(encoder);
}

fn pallette_to_rgb(palette: &[u8]) -> Vec<RGB<u8>>
{
    palette
        .chunks(3)
        .map(|x| RGB::from_iter(x.to_owned()))
        .collect()
}
