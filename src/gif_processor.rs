use std::borrow::Cow;
use std::convert::TryInto;
use std::io::Read;

use gif;

const SCALE: f64 = 1.2;

// Find the indices of the minimum and maximum values in one
// iteration, panics if thes's no element.
fn minmax_ids<I, E>(mut it: I) -> (usize, usize)
where
    I: Iterator<Item = E>,
    E: Ord + Copy,
{
    let mut max = match it.next() {
        Some(val) => val,
        None => {
            panic!("Empty array has no minmax");
        }
    };
    let (mut min_idx, mut max_idx) = (0, 0);
    let mut pos = 1;
    let mut min = max;
    while let Some(x) = it.next() {
        if x > max {
            max_idx = pos;
            max = x;
        } else if x < min {
            min_idx = pos;
            min = x;
        }
        pos += 1;
    }
    (min_idx, max_idx)
}

// Generate the section to prepend by fitting some text into
// the designated area.
fn make_prepend(
    width: u16,
    height: u16,
    black: u8,
    white: u8,
    _text: String,
) -> Vec<u8>
{
    let length = width as usize * height as usize;
    let mut buf = vec![white; length];

    buf
}

//#[test]
//pub fn caption_test() -> ()
pub fn caption<R: Read>(_name: &String, bytes: R, caption: &String) -> Vec<u8>
{
    //let input = File::open("test.gif").unwrap();
    //let mut out_image = File::create("result.gif").unwrap();
    let mut out_image = Vec::new();

    let mut options = gif::DecodeOptions::new();
    // This should be the default anyway, but better safe
    options.set_color_output(gif::ColorOutput::Indexed);
    let mut decoder = options.read_info(bytes).unwrap();

    let h = decoder.height();
    let w = decoder.width();
    let new_h = (h as f64 * SCALE) as u16;

    let (palette, min, max): (Vec<u8>, usize, usize) =
        match decoder.global_palette() {
            Some(palette) => {
                //if a global palette exists, we search for the darkest
                //and lightest colours to use as black and white
                let sums = palette
                    .chunks(3)
                    // take the sums of each chunk of 3 bytes (rgb) to
                    // represent its total brightness
                    .map(|x| x.iter().map(|y| *y as usize).sum::<usize>());

                let (min, max) = minmax_ids(sums);

                //println!("min {}, max {}", min, max);
                (palette.to_vec(), min, max)
            }
            None => {
                //global palette needs to have 256 colours, each taking
                //3 bytes for r, g, b - so make 255 black and add white
                let mut p = vec![0; 255 * 3];
                p.extend([1; 3]);
                (p, 0, 255)
            }
        };

    let pre = make_prepend(
        w,
        new_h - h,
        min.try_into().unwrap(),
        max.try_into().unwrap(),
        caption.to_string(),
    );

    //println!("w {}, h {}, new_h {}, pre {}", w, h, new_h, pre.len());

    let mut encoder =
        gif::Encoder::new(&mut out_image, w, new_h, palette.as_slice())
            .unwrap();
    encoder.set_repeat(gif::Repeat::Infinite).unwrap();

    while let Some(old_frame) = decoder.read_next_frame().unwrap() {
        let mut new_frame = old_frame.clone();
        new_frame.width = w;
        new_frame.height = new_h;
        process_buffer(&w, &new_h, &pre.as_slice(), &mut new_frame.buffer); //, &mut frame.buffer);

        encoder.write_frame(&new_frame).unwrap();
    }
    drop(encoder);
    out_image
}

// All this does is take the old buffer, concat it with the prependix
// create a new frame for that and move its buffer into the old one.
// Other features may be added later.
fn process_buffer(
    width: &u16,
    height: &u16,
    pre: &[u8],
    buffer: &mut Cow<[u8]>,
) -> ()
{
    let new = gif::Frame::from_indexed_pixels(
        *width,
        *height,
        &[pre, buffer].concat(),
        None,
    );
    *buffer = new.buffer;
}
