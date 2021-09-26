use std::fs::File;
use std::io::Write;

use crate::gif_processor;

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
