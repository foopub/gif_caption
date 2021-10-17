use std::io::Read;

use fontdue::layout::{
    CoordinateSystem, HorizontalAlign, Layout, LayoutSettings, TextStyle,
    VerticalAlign, WrapStyle,
};
use fontdue::{Font, FontSettings};
use gif::{ColorOutput, DecodeOptions, DisposalMethod, Encoder, Repeat};
use rgb::RGB;
use wu_quantization::compress;

const SCALE: f32 = 0.3;

#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub enum ColourCompression
{
    // Wu with number of colours
    Wu(u8),
    None,
}

impl Default for ColourCompression
{
    fn default() -> Self
    {
        Self::None
    }
}

#[allow(dead_code)]
pub enum Indexer
{
    // Wu indexer from rgb
    Wu(Box<dyn Fn([u8; 3]) -> u8>),
    // Deduped old idx to new idx
    Deduped(Box<dyn Fn(u8) -> u8>),
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
    layout.append(&[&font], &TextStyle::new(text, px, 0));
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
            // get x, y coordinate and draw pixel...
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

pub fn palette_to_rgb(palette: &[u8]) -> Vec<RGB<u8>>
{
    palette
        .chunks(3)
        .map(|x| RGB::new(x[0], x[1], x[2]))
        .collect()
}

fn process_palatte<R>(
    mut decoder: gif::Decoder<R>,
    comprssion: ColourCompression,
) -> (Vec<u8>, Indexer)
where
    //T: Fn(u8) -> u8,
    R: Read + Copy,
{
    // default global palette is just black and white
    let global_palette = decoder
        .global_palette()
        .unwrap_or(&[255, 255, 255, 0, 0, 0])
        .to_vec();

    if let ColourCompression::Wu(number) = comprssion {
        let mut all_colours = palette_to_rgb(&global_palette);

        while let Some(frame) = decoder.next_frame_info().unwrap() {
            if let Some(p) = &frame.palette {
                all_colours.extend(palette_to_rgb(p));
            }
        }

        // if combined palette does not exceed number, it's better to do nothing
        let mut unique = all_colours.clone();
        unique.sort_unstable();
        unique.dedup();

        if unique.len() > number as usize {
            drop(unique);
            drop(global_palette);
            let (p, i) = compress(all_colours, number as usize);
            return (
                p,
                Indexer::Wu(Box::new(move |x| {
                    *i.rgb_index(RGB::new(x[0] >> 3, x[1] >> 3, x[2] >> 3))
                })),
            );
        }
    }
    //TODO if colour count is small enough, we can add black/white
    //TODO global palette can often be dedupped
    (global_palette.to_vec(), Indexer::None)
}

pub fn caption<R: Read + Copy>(
    _name: &str,
    bytes: R,
    caption: &str,
    compression: ColourCompression,
    scale: Option<f32>,
    font_size: Option<f32>,
    //smooth_font: bool, TODO
) -> Vec<u8>
{
    let mut decoder_opts = DecodeOptions::new();
    decoder_opts.set_color_output(ColorOutput::RGBA);
    let decoder = decoder_opts.read_info(bytes).unwrap();

    let w = decoder.width();
    let old_h = decoder.height();

    // global palette and optional indexer if compressed
    let (global_palette, indexer) = process_palatte(decoder, compression);

    let (h, piece) = {
        let piece_height = (old_h as f32 * scale.unwrap_or(SCALE)) as u16;
        let h = old_h + piece_height;

        // if a px_size is provided, we use that, otherwise we calculate it
        // from the area per char available (tho it's better to use graphemes)
        let px = {
            if let Some(px) = font_size {
                px
            } else {
                let n_chars = caption.chars().count() as f32;
                let text_area = w as usize * piece_height as usize;
                let area_per_char = text_area as f32 / n_chars;
                area_per_char.sqrt() as f32
            }
        };

        let piece = match &indexer {
            Indexer::Wu(indexer) => make_piece(
                w,
                piece_height,
                px,
                |x| indexer([255 - x; 3]),
                caption,
            ),
            Indexer::Deduped(indexer) => {
                make_piece(w, piece_height, px, indexer, caption)
            }
            Indexer::None => {
                let basic_indexer = {
                    let (black, white) = minmax_ids(
                        // sum chunks of 3 to get the absolute brightness
                        global_palette.chunks(3).map(|x| {
                            x.iter().map(|y| *y as usize).sum::<usize>()
                        }),
                    );
                    move |x| if x > 30 { black as u8 } else { white as u8 }
                };
                make_piece(w, piece_height, px, basic_indexer, caption)
            }
        };

        (h, piece)
    };
    let shift_h = h - old_h;

    let mut out_image = Vec::new();
    let mut encoder =
        { Encoder::new(&mut out_image, w, h, &global_palette).unwrap() };
    encoder.set_repeat(Repeat::Infinite).unwrap();

    let mut decoder_opts = DecodeOptions::new();

    match indexer {
        Indexer::Wu(indexer) => {
            decoder_opts.set_color_output(ColorOutput::RGBA);
            let mut decoder = decoder_opts.read_info(bytes).unwrap();

            while let Some(old_frame) = decoder.read_next_frame().unwrap() {
                let mut new_frame = old_frame.clone();

                let triplets: Vec<[u8; 3]> = old_frame
                    .buffer
                    .chunks_exact(4)
                    .map(|x| [x[0], x[1], x[2]])
                    .collect();

                let mut new_buff = Vec::with_capacity(triplets.len());

                triplets.iter().for_each(|x| {
                    new_buff.push(indexer(*x));
                });

                new_frame.palette = None;
                new_frame.height = h;
                new_frame.buffer = [piece.clone(), new_buff].concat().into();
                encoder.write_frame(&new_frame).unwrap();
            }
        }
        Indexer::Deduped(_) => todo!(),
        Indexer::None => {
            decoder_opts.set_color_output(ColorOutput::Indexed);
            let mut decoder = decoder_opts.read_info(bytes).unwrap();

            // well... I learned about disposal methods smh
            let mut previous_disposal = DisposalMethod::Background;

            while let Some(old_frame) = decoder.read_next_frame().unwrap() {
                let mut new_frame = old_frame.clone();

                // if the disposal method is not Keep, we need to re-add piece
                match previous_disposal {
                    DisposalMethod::Keep | DisposalMethod::Previous => {
                        new_frame.top += shift_h;
                    }
                    _ => {
                        // TODO if frame uses local palette, colours need to be
                        // adjusted... this is a pain because, for consistent results, every 
                        // local palette needs to be processed to make sure it either has
                        // black and white, or less than 255 colours. 
                        //
                        // Alternatively the piece can be made transparent?????
                        new_frame.height = h;
                        new_frame.buffer =
                            [piece.as_ref(), new_frame.buffer.as_ref()]
                                .concat()
                                .into();
                    }
                }
                previous_disposal = new_frame.dispose;
                encoder.write_frame(&new_frame).unwrap();
            }
        }
    }
    drop(encoder);
    out_image
}
