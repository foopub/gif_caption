use std::fs;

use criterion::{criterion_group, criterion_main, Criterion};
use gif::{Encoder, Frame, Repeat};
use png;

const DIR: &str = "benches/samples";

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
                c.bench_function("default", |b| {
                    b.iter(|| Frame::from_rgb(w, h, &mut buf))
                });
                Frame::from_rgb(w, h, &mut buf[..size])
            }
            png::ColorType::Rgba => {
                c.bench_function("default", |b| {
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
