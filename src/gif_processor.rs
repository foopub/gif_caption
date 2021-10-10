use std::borrow::Cow;
//use std::convert::TryInto;
use std::io::Read;

use fontdue::layout::{
    CoordinateSystem, HorizontalAlign, Layout, LayoutSettings, TextStyle,
    VerticalAlign, WrapStyle,
};
use fontdue::{Font, FontSettings};
use gif::{ColorOutput, DecodeOptions, DisposalMethod, Encoder, Repeat};
use rgb::RGB;

const SCALE: f32 = 1.3;

#[allow(dead_code)]
pub enum CompressColours
{
    WuColours(u8), // number of colours
    NQSpeed(u8),   // speed
    None,
}

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
    for x in it {
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
fn make_piece<T>(
    piece_width: u16,
    piece_height: u16,
    px: f32,
    palette_idx: T,
    text: &str,
) -> Vec<u8>
where
    T: Fn(u8) -> u8,
{
    // TODO if px is too big or too small, change scale OR increase layout
    // size

    // prepare the font, layout, and canvas ~~~~~~~~~~~~~~~~~~~~~~~~~
    // font
    let font = {
        let font = include_bytes!("../fonts/FjallaOne-Regular.ttf");
        Font::from_bytes(
            font.as_ref(),
            FontSettings {
                collection_index: 0,
                scale: px,
            },
        )
        .unwrap()
    };
    //layout
    let mut layout = Layout::new(CoordinateSystem::PositiveYDown);
    layout.reset(&LayoutSettings {
        x: 0.0,
        y: 0.0,
        max_width: Some(piece_width.into()),
        max_height: Some(piece_height.into()),
        horizontal_align: HorizontalAlign::Center,
        vertical_align: VerticalAlign::Middle,
        wrap_style: WrapStyle::Word,
        wrap_hard_breaks: true,
    });
    // canvas
    let mut canvas =
        vec![palette_idx(0); piece_width as usize * piece_height as usize];

    // "write" text to the layout
    layout.append(&[&font], &TextStyle::new(&text, px, 0));
    //println!("Creating pre {:#?}", layout.glyphs());

    // now draw 🔫 ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    for glyph in layout.glyphs() {
        let (mut x, mut y, w) = (
            glyph.x as usize,
            glyph.y as usize,
            glyph.width,
            //glyph.height,
        );

        let x0 = x;
        let w = w + x;
        let (_, bitmap) =
            &font.rasterize_indexed(glyph.key.glyph_index as usize, px);

        //println!("{}, {}, {}, {}, {}", x, y, bitmap.len(), w, h);
        for pixel in bitmap {
            // when position exceeds piece width limit, reset x and move
            // down to next line
            if x > (piece_width - 1).into() {
                x = x0;
                y += 1;
                continue;
            }
            // when position exceeds y limit, there's nothing more to draw
            if y > (piece_height - 1).into() {
                break;
            }
            // get x, y coordinate and draw black/white pixel...
            // TODO add grayscale and potentially colour
            canvas[x + y * piece_width as usize] = palette_idx(*pixel);

            // advance x
            x += 1;
            if x == w {
                x = x0;
                y += 1;
            }
        }
    }

    canvas
}

fn palette_to_rgb(palette: &[u8]) -> Vec<RGB<u8>>
{
    palette
        .chunks(3)
        .map(|x| RGB::new(x[0], x[1], x[2]))
        .collect()
}

pub fn caption<R: Read + Copy>(
    _name: &str,
    bytes: R,
    caption: &str,
    compress: CompressColours,
    scale: Option<f32>,
    font_size: Option<f32>,
) -> Vec<u8>
{
    let mut options = DecodeOptions::new();
    options.set_color_output(ColorOutput::RGBA);
    let mut decoder = options.read_info(bytes).unwrap();

    let (global_palette, indexer) = {
        if let CompressColours::WuColours(number) = compress {
            let mut all_colours = Vec::new();
            // default global palette is just black and white
            let mut global_palette = vec![255, 255, 255, 0, 0, 0];

            if let Some(p) = decoder.global_palette() {
                all_colours.extend(palette_to_rgb(p));
                // if the gif has a global palette, overwrite the default
                global_palette = p.to_vec();
            }
            while let Some(frame) = decoder.next_frame_info().unwrap() {
                if let Some(p) = &frame.palette {
                    all_colours.extend(palette_to_rgb(p));
                }
            }

            // if combined palette does not exceed number, there's nothing to do
            let mut unique = all_colours.clone();
            unique.sort_unstable();
            unique.dedup();
            if unique.len() < number as usize {
                //TODO if colour count is small enough, we can add black/white
                (global_palette, None)
            } else {
                drop(unique);
                drop(global_palette);
                let (p, i) =
                    crate::clustering::compress(all_colours, number as usize);
                (p, Some(i))
            }
        } else {
            if let Some(p) = decoder.global_palette() {
                (p.to_vec(), None)
            } else {
                (vec![255, 255, 255, 0, 0, 0], None)
            }
        }
    };

    let w = decoder.width();

    let (h, pre) = {
        // calculate all the dimensions ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
        // default extension is 30%
        let old_h = decoder.height();
        let piece_height =
            (old_h as f32 * (scale.unwrap_or(SCALE) - 1.0)) as u16;

        let h = old_h + piece_height;

        let px = {
            // if a px_size is provided, we use that
            if let Some(px) = font_size {
                px
            } else {
                let text_area = w as usize * piece_height as usize;
                // only support chars, not graphemes for now;
                let n_chars = caption.chars().count() as f32;
                //test gif is 225x420 btw
                let area_per_char = text_area as f32 / n_chars;
                // some arbitrary ratio
                // area_per_char = area_per_char * (1.0 / CHAR_RATIO);
                area_per_char.sqrt() as f32
            }
        };

        let pre = if let Some(idx) = &indexer {
            make_piece(
                w,
                piece_height,
                px,
                |x| *idx.index(RGB::from([(255 - x) >> 3; 3])),
                caption,
            )
        } else {
            let (black, white) = minmax_ids(
                // sum chunks of 3 to get the absolute brightness
                global_palette
                    .chunks(3)
                    .map(|x| x.iter().map(|y| *y as usize).sum::<usize>()),
            );
            make_piece(
                w,
                piece_height,
                px,
                |x| if x > 30 { black as u8 } else { white as u8 },
                caption,
            )
        };

        (h, pre)
    };
    let shift_h = h - decoder.height();

    let mut out_image = Vec::new();
    // if the palette is global, use it, else we need to pass an empty vec
    let mut encoder =
        { Encoder::new(&mut out_image, w, h, &global_palette).unwrap() };
    encoder.set_repeat(Repeat::Infinite).unwrap();

    // well... I learned about disposal methods smh
    let mut previous_disposal = DisposalMethod::Background;

    let mut options = DecodeOptions::new();

    if let Some(idx) = indexer {
        options.set_color_output(ColorOutput::RGBA);

        let mut decoder = options.read_info(bytes).unwrap();

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
                new_buff.push(*idx.index(RGB::new(
                    x[0] >> 3,
                    x[1] >> 3,
                    x[2] >> 3,
                )));
            });
            //println!("{}", new_buff.len());

            new_frame.height = h;
            new_frame.buffer = [pre.clone(), new_buff].concat().into();
            encoder.write_frame(&new_frame).unwrap();
        }
    } else {
        let mut decoder = options.read_info(bytes).unwrap();

        while let Some(old_frame) = decoder.read_next_frame().unwrap() {
            let mut new_frame = old_frame.clone();
            // if the disposal method is not Keep, we need to re-add the piece
            match previous_disposal {
                DisposalMethod::Keep | DisposalMethod::Previous => {
                    new_frame.top += shift_h;
                }
                _ => {
                    // TODO if frame uses local palette, colours need to be
                    // adjusted
                    new_frame.height = h;
                    process_indexed(w, h, pre.as_slice(), &mut new_frame.buffer);
                }
            }
            previous_disposal = new_frame.dispose;
            encoder.write_frame(&new_frame).unwrap();
        }
    }
    drop(encoder);
    out_image
}

// All this does is take the old buffer, concat it with the prependix
// create a new frame for that and move its buffer into the old one.
// Other features may be added later.
fn process_indexed(width: u16, height: u16, pre: &[u8], buffer: &mut Cow<[u8]>)
{
    let new = gif::Frame::from_indexed_pixels(
        width,
        height,
        &[pre, buffer].concat(),
        None,
    );
    *buffer = new.buffer;
}
