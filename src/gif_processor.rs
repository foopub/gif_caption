use std::borrow::Cow;
use std::convert::TryInto;
use std::io::Read;

const SCALE: f64 = 1.3;

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

enum Palette
{
    GlobalPalette(Vec<u8>),
    LocalPalette(Vec<u8>),
}

// Oricess the supplied global palette to find the darkest and
// lightest colours, and if none, create a local palette with
// just black and white
fn process_palette(palette: Option<&[u8]>) -> (Palette, u8, u8)
{
    let (palette, min, max): (Palette, usize, usize) =
        if let Some(palette) = palette {
            // if a global palette exists, we search for the darkest
            // and lightest colours to use as black and white
            let sums = palette
                .chunks(3)
                // take the sums of each chunk of 3 bytes (rgb) to
                // represent its total brightness
                .map(|x| x.iter().map(|y| *y as usize).sum::<usize>());

            let (min, max) = minmax_ids(sums);
            //println!("min {}, max {}", min, max);
            let mut p = palette.to_vec();
            p[min * 3..min * 3 + 3].fill(0);
            p[max * 3..max * 3 + 3].fill(255);
            //println!("Global palette");
            (Palette::GlobalPalette(p), min, max)
        } else {
            // if no global palette, we can pass a local palette for
            // the first frame only
            let mut p = vec![0; 3];
            p.extend([255; 3]);
            //println!("Local palette");
            (Palette::LocalPalette(p), 0, 1)
        };
    (palette, min.try_into().unwrap(), max.try_into().unwrap())
}

// Generate the section to prepend by fitting some text into
// the designated area.
fn make_prepend(
    piece_width: u16,
    total_height: u16,
    //black: u8,
    //white: u8,
    palette: Option<&[u8]>,
    text: String,
) -> (u16, Vec<u8>, Palette)
{
    use fontdue::layout::{
        CoordinateSystem, HorizontalAlign, Layout, LayoutSettings, TextStyle,
        VerticalAlign, WrapStyle,
    };
    use fontdue::{Font, FontSettings};

    let (palette, black, white) = process_palette(palette);

    // calculate all the dimensions ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    // default extension is 30%
    let piece_height = (total_height as f64 * (SCALE - 1.0)) as u16;
    let text_area = piece_width as f64 * piece_height as f64;

    let px = {
        // only support chars, not graphemes for now;
        let n_chars = text.chars().count() as f64;
        //test gif is 225x420 btw
        let area_per_char = text_area / n_chars;
        // some arbitrary ratio
        // area_per_char = area_per_char * (1.0 / CHAR_RATIO);
        area_per_char.sqrt() as f32
    };

    // TODO if px is too big or too small, change scale
    //println!("Piece w {}, h {}, px {}", piece_width, piece_height, px);

    // prepare the font, layout, and canvas ~~~~~~~~~~~~~~~~~~~~~~~~~
    // font lol
    let font = include_bytes!("../fonts/FjallaOne-Regular.ttf");
    let font = Font::from_bytes(
        font.as_ref(),
        FontSettings {
            collection_index: 0,
            scale: px,
        },
    )
    .unwrap();
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
    let mut canvas = vec![white; text_area as usize];

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
            canvas[x + y * piece_width as usize] =
                if *pixel < 128 { white } else { black };

            // advance x
            x += 1;
            if x == w {
                x = x0;
                y += 1;
            }
        }
    }

    (total_height + piece_height, canvas, palette)
}

pub fn caption<R: Read>(_name: &str, bytes: R, caption: &str) -> Vec<u8>
{
    use gif::{ColorOutput, DecodeOptions, DisposalMethod, Encoder, Repeat};

    let mut out_image = Vec::new();

    let mut options = DecodeOptions::new();
    // This should be the default anyway, but better safe
    options.set_color_output(ColorOutput::Indexed);
    let mut decoder = options.read_info(bytes).unwrap();

    let old_h = decoder.height();
    let w = decoder.width();

    // new height!
    let (h, pre, palette) = make_prepend(
        w,
        old_h,
        decoder.global_palette(),
        //min.try_into().unwrap(),
        //max.try_into().unwrap(),
        caption.to_string(),
    );
    let h_shift = h - old_h;

    //println!("w {}, h {}, new_h {}, pre {}", w, h, new_h, pre.len());

    // if the palette is global, use it, else pass an empty slice
    let mut encoder = {
        let global_palette = match palette {
            Palette::GlobalPalette(p) => p,
            Palette::LocalPalette(_) => vec![],
        };
        // empty vec means no global palette
        Encoder::new(&mut out_image, w, h, &global_palette).unwrap()
    };
    encoder.set_repeat(Repeat::Infinite).unwrap();

    // well... I learned about disposal methods smh
    let mut previous_disposal = DisposalMethod::Background;

    while let Some(old_frame) = decoder.read_next_frame().unwrap() {
        let mut new_frame = old_frame.clone();
        // if the disposal method is not Keep, we need to re-add the piece
        //println!("{:?}", new_frame.dispose);
        match previous_disposal {
            DisposalMethod::Keep | DisposalMethod::Previous => {
                new_frame.top += h_shift;
            }
            _ => {
                // TODO if frame uses local palette, colours need to be adjusted
                new_frame.height = h;
                process_buffer(w, h, pre.as_slice(), &mut new_frame.buffer);
            }
        }
        previous_disposal = new_frame.dispose;
        encoder.write_frame(&new_frame).unwrap();
    }
    drop(encoder);
    out_image
}

// All this does is take the old buffer, concat it with the prependix
// create a new frame for that and move its buffer into the old one.
// Other features may be added later.
fn process_buffer(width: u16, height: u16, pre: &[u8], buffer: &mut Cow<[u8]>)
{
    let new = gif::Frame::from_indexed_pixels(
        width,
        height,
        &[pre, buffer].concat(),
        None,
    );
    *buffer = new.buffer;
}
