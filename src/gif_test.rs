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
        &String::from(" "),
        gif_processor::ColourCompression::Wu(8),
        None,
        None,
        //Some(60.0),
    );
    out_image.write(&out).unwrap();
}
