use std::fs::File;
use std::io::{Read, Write};

use crate::gif_processor;

#[test]
fn sample_gifs()
{
    let mut buf = Vec::new();
    {
        let mut input = File::open("test.gif").unwrap();
        input.read_to_end(&mut buf).unwrap();
    }
    let mut out_image = File::create("result.gif").unwrap();
    let out = gif_processor::caption(
        "test",
        buf.as_slice(),
        &String::from("my descent into madness is complete"),
        gif_processor::CompressColours::WuColours(255),
        None,
        None,
        //Some(60.0),
    );
    out_image.write(&out).unwrap();
}
