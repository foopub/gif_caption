use std::fs;
use std::borrow::Cow;

use criterion::{criterion_group, criterion_main, Criterion};
use gif::{Encoder, Frame, Repeat};
use gif_caption::gif_processor::{palette_to_rgb, Indexer};
use png;
use rgb::RGB;
use wu_quantization::compress;

const DIR: &str = "benches/samples";

fn wu_compression(palette: &[u8], n_colours: usize) -> (Vec<u8>, Indexer)
{
    let mut palette_vec = palette_to_rgb(palette);
    palette_vec.sort_unstable();
    palette_vec.dedup();
    if palette_vec.len() > n_colours {
        let (p, i) = compress(palette_vec, n_colours);
        return (
            p,
            Indexer::Wu(Box::new(move |x| {
                *i.rgb_index(RGB::new(x[0] >> 3, x[1] >> 3, x[2] >> 3))
            })),
        );
    } else {
        todo!()
    }
}

fn rgb_frame(width: u16, height: u16, pixels: &mut [u8]) -> Frame<'static>
{
    let (palette, indexer) = wu_compression(pixels, 256);

    let triplets: Vec<[u8; 3]> = pixels.chunks_exact(3).map(|x| [x[0], x[1], x[2]]).collect();

    //this is useless
    let transparent = triplets.iter().find(|x| x[2] as usize > 1000);

    let mut buffer = Vec::with_capacity(triplets.len());

    if let Indexer::Wu(indexer) = indexer {
        triplets.iter().for_each(|x| {
            buffer.push(indexer(*x));
        });

        Frame {
            width,
            height,
            buffer: Cow::Owned(buffer),
            palette: Some(palette),
            transparent: transparent.map(|t| indexer(*t)),
            ..Frame::default()
        }
    } else {
        todo!()
    }
}


fn default(c: &mut Criterion)
{
    let dir = fs::read_dir(DIR).expect("Cant'r read dir:\n{}");
    for path in dir {
        let path = path.expect("Can't read path:\n{}").path();
        if path.extension().unwrap() != "png" {
            continue;
        }

        let mut reader = {
            let input = fs::File::open(&path).unwrap();
            let decoder = png::Decoder::new(input);
            decoder.read_info().unwrap()
        };

        let mut buf = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buf).unwrap();

        let (w, h, size) = {
            // could use try_into().unwrap() but probably no need
            (info.width as u16, info.height as u16, info.buffer_size())
        };

        let mut encoder = {
            let output = fs::File::create(path.with_extension("gif")).unwrap();
            Encoder::new(output, w, h, &[]).unwrap()
        };
        encoder.set_repeat(Repeat::Finite(0)).unwrap();

        //println!("{}, {}, {}, {}", reader.output_buffer_size(), size, w, h);
        let frame = match info.color_type {
            png::ColorType::Rgb => {
                c.bench_function("default_rgb", |b| {
                    b.iter(|| Frame::from_rgb(w, h, &mut buf))
                });
                c.bench_function("wu_algo_rgb", |b| {
                    b.iter(|| rgb_frame(w, h, &mut buf))
                });
                //Frame::from_rgb(w, h, &mut buf[..size])
                rgb_frame(w, h, &mut buf[..size])
            }
            png::ColorType::Rgba => {
                c.bench_function("default_rgba", |b| {
                    b.iter(|| Frame::from_rgba(w, h, &mut buf))
                });
                Frame::from_rgba(w, h, &mut buf[..size])
            }
            c => {
                println!("Frame has ColourType: {:?}", c);
                continue;
            }
        };

        encoder.write_frame(&frame).unwrap();
    }
}

criterion_group!(benches, default);
criterion_main!(benches);
