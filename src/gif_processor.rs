use std::borrow::Cow;
use std::convert::TryInto;
use std::io::Read;

use {fontdue, gif};

const SCALE: f64 = 1.3;
const CHAR_RATIO: f64 = 0.8;

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
    piece_width: u16,
    total_height: u16,
    black: u8,
    white: u8,
    text: String,
) -> (u16, Vec<u8>)
{
    use fontdue::layout::{
        CoordinateSystem, HorizontalAlign, Layout, LayoutSettings, TextStyle,
        VerticalAlign, WrapStyle,
    };
    use fontdue::{Font, FontSettings};


    // Default extension is 30% and allows 80 characters of
    // text total.
    let piece_height = (total_height as f64 * (SCALE - 1.0)) as u16;
    let text_area = piece_width as f64 * piece_height as f64;

    let px = {
        //only support chars, not graphemes for now;
        let n_chars = text.chars().count() as f64;
        //test gif is 225x420 btw
        let area_per_char = text_area / n_chars;
        // some arbitrary ratio
        //area_per_char = area_per_char * (1.0 / CHAR_RATIO);
        area_per_char.sqrt() as f32
    };

    println!("px {}", px);
    //if px is too big or too small, change scale
    println!("Piece width, height {}, {}", piece_width, piece_height);

    let font = include_bytes!("../fonts/FjallaOne-Regular.ttf");
    let font = Font::from_bytes(
        font.as_ref(),
        FontSettings {
            collection_index: 0,
            scale: px,
        },
    )
    .unwrap();

    let mut layout = Layout::new(CoordinateSystem::PositiveYDown);
    layout.reset(&LayoutSettings {
        x: 0.0,
        y: 0.0,
        max_width: Some(piece_width.into()),
        max_height: Some(piece_height.into()),
        horizontal_align: HorizontalAlign::Center,
        vertical_align: VerticalAlign::Top,
        wrap_style: WrapStyle::Word,
        wrap_hard_breaks: true,
    });

    //
    layout.append(&[&font], &TextStyle::new(&text, px, 0));

    //make the canvas
    let mut buf = vec![white; text_area as usize];

    //println!("Creating pre {:#?}", layout.glyphs());
    for glyph in layout.glyphs() {
        let (mut x, mut y, w, h) = (
            glyph.x as usize,
            glyph.y as usize,
            glyph.width,
            glyph.height,
        );
        
        let x0 = x.clone();
        let w = w + x;
        let (_, bitmap) =
            &font.rasterize_indexed(glyph.key.glyph_index as usize, px);
        println!("{}, {}, {}, {}, {}", x, y, bitmap.len(), w, h);
        for pixel in bitmap {
            if x > (piece_width - 1).into() || y > (piece_height-1).into() {
                x += 1;
                if x == w {
                    x = x0;
                    y += 1;
                }
                continue;
            }

            buf[x + y * piece_width as usize] =
                if *pixel < 128 { white } else { black };

            //advance x
            x += 1;
            if x == w {
                x = x0;
                y += 1;
            }
        }
        //println!("{:?}", buf)
    }
    (total_height + piece_height, buf)
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

    // new height!
    let (new_h, pre) = make_prepend(
        w,
        h,
        min.try_into().unwrap(),
        max.try_into().unwrap(),
        caption.to_string(),
    );
    let h_shift = new_h - h;
    let h = new_h;

    //println!("w {}, h {}, new_h {}, pre {}", w, h, new_h, pre.len());

    let mut encoder =
        gif::Encoder::new(&mut out_image, w, h, palette.as_slice()).unwrap();
    encoder.set_repeat(gif::Repeat::Infinite).unwrap();

    //we only really need to process the first frame for now
    if let Some(old_frame) = decoder.read_next_frame().unwrap() {
        let mut new_frame = old_frame.clone();
        new_frame.width = w;
        new_frame.height = h;
        process_buffer(&w, &h, &pre.as_slice(), &mut new_frame.buffer); //, &mut frame.buffer);

        encoder.write_frame(&new_frame).unwrap();
    }

    while let Some(old_frame) = decoder.read_next_frame().unwrap() {
        let mut new_frame = old_frame.clone();
        new_frame.top += h_shift;
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
