// colours have 3 dimesions with weight
// methods to try
// Wu's - very fast, looks better than NQ
// https://doi.org/10.1016/B978-0-08-050754-5.50035-9
//
// BS-ATCQ - very good quality, medium speed
// BKMS - best quality, slowest
// Fast nearest neighbour - looks very promising too

use rgb::RGB;

//type u8 = u8;
const COLOURS: usize = 256;
const ROUND_N: usize = 3;
const SPACE_SIZE: usize = 255 >> ROUND_N;
const UNIT_RGB: RGB<u8> = RGB::new(1u8, 1, 1);

struct ColourSpace
{
    s: [[[ColourEntry; SPACE_SIZE]; SPACE_SIZE]; SPACE_SIZE],
}

#[derive(Copy, Clone)]
struct ColourEntry
{
    pub m: RGB<u8>,
    pub count: usize,
    pub m2: usize,
}

struct ColourCube
{
    end: RGB<u8>,
    start: RGB<u8>,
}

impl ColourEntry
{
    fn new() -> Self
    {
        let m = RGB::new(0u8, 0, 0);
        let (count, m2) = (0, 0);
        ColourEntry { m, count, m2 }
    }
    fn add_some(&mut self, other: &Self) -> ()
    {
        self.m.add_inplace(&other.m);
        self.count += other.count;
    }
    fn add_inplace(&mut self, other: &Self) -> ()
    {
        self.add_some(&other);
        self.m2 += other.m2;
    }
    fn sub_some(&mut self, other: &Self) -> ()
    {
        self.m.sub_inplace(&other.m);
        self.count -= other.count;
    }
    fn sub_inplace(&mut self, other: &Self) -> ()
    {
        self.sub_some(&other);
        self.m2 -= other.m2;
    }
    fn sub(&self, other: &Self) -> Self
    {
        let mut out = self.clone();
        out.m.sub_inplace(&other.m);
        out.count -= other.count;
        out.m2 -= other.m2;
        out
    }
}

impl<T: Into<usize>> From<(&RGB<u8>, T, T)> for ColourEntry
{
    fn from(entry_tuple: (&RGB<u8>, T, T)) -> Self
    {
        ColourEntry {
            m: *entry_tuple.0,
            count: entry_tuple.1.into(),
            m2: entry_tuple.2.into(),
        }
    }
}

impl ColourSpace
{
    fn new() -> ColourSpace
    {
        let entry = ColourEntry::new();
        let s = [[[entry; SPACE_SIZE]; SPACE_SIZE]; SPACE_SIZE];
        ColourSpace { s }
    }
    fn index(&self, rgb: &[u8; 3]) -> &ColourEntry
    {
        &self.s[rgb[0] as usize][rgb[1] as usize][rgb[2] as usize]
    }
    fn index_mut(&mut self, rgb: &[u8; 3]) -> &mut ColourEntry
    {
        &mut self.s[rgb[0] as usize][rgb[1] as usize][rgb[2] as usize]
    }
}

trait Wu
{
    fn dominates(&self, other: Self) -> bool;
    fn round(&self) -> [u8; 3];
    fn squared(&self) -> usize;
    fn add_inplace(&mut self, other: &Self) -> ();
    fn sub_inplace(&mut self, other: &Self) -> ();
    fn sub(&self, other: &Self) -> Self;
}

impl Wu for RGB<u8>
{
    fn dominates(&self, other: Self) -> bool
    {
        (self.r > other.r) & (self.g > other.g) & (self.b > other.b)
    }
    fn round(&self) -> [u8; 3]
    {
        //round up the bytes by ignoring the last few bits
        [self.r >> ROUND_N, self.g >> ROUND_N, self.b >> ROUND_N]
    }
    fn squared(&self) -> usize
    {
        self.iter().map(|x| x as usize ^ 2).sum()
    }
    fn add_inplace(&mut self, other: &Self) -> ()
    {
        self.r += other.r;
        self.g += other.g;
        self.b += other.b;
    }
    fn sub_inplace(&mut self, other: &Self) -> ()
    {
        self.r -= other.r;
        self.g -= other.g;
        self.b -= other.b;
    }
    fn sub(&self, other: &Self) -> Self
    {
        let mut out = self.clone();
        out.r -= other.r;
        out.g -= other.g;
        out.b -= other.b;
        out
    }
}

/// Iterate over the palette and sum each pixel colour by rgb,
/// P(c) and c^2
fn histogram(palette: &[RGB<u8>], space: &mut ColourSpace) -> ()
{
    // do everything in one iteration over the palette
    palette
        .iter()
        .map(|pixel| (pixel.round(), pixel))
        .for_each(|(idx, p)| {
            let s = space.index_mut(&idx);
            s.add_inplace(&ColourEntry::from((p, 1, p.squared())));
        });
}

fn cummulate_vals(space: &mut ColourSpace) -> ()
{
    for r in 0..SPACE_SIZE {
        let mut area = [ColourEntry::new(); SPACE_SIZE];

        for g in 0..SPACE_SIZE {
            let mut line = ColourEntry::new();

            for b in 0..SPACE_SIZE {
                let mut point = space.s[r][g][b];

                line.add_inplace(&point);
                area[b].add_inplace(&line);
                point.add_inplace(&area[b]);
                point.add_inplace(&space.s[r - 1][g][b]);
            }
        }
    }
}

#[derive(Copy, Clone)]
enum Direction
{
    Red,
    Green,
    Blue,
}

//fn combine_all(
//    pos: &[[u8; 3]],
//    neg: &[[u8; 3]],
//    space: &ColourSpace,
//    entry: &mut ColourEntry,
//) -> ()
//{
//    pos.iter()
//        .filter(|x| !x.contains(&(SPACE_SIZE as u8 + 1)))
//        .for_each(|x| entry.add_inplace(space.index(x)));
//    neg.iter()
//        .filter(|x| !x.contains(&(SPACE_SIZE as u8 + 1)))
//        .for_each(|x| entry.sub_inplace(space.index(x)));
//}

fn combine_some(
    pos: &[[u8; 3]],
    neg: &[[u8; 3]],
    space: &ColourSpace,
    entry: &mut ColourEntry,
) -> ()
{
    pos.iter()
        .filter(|x| !x.contains(&(SPACE_SIZE as u8 + 1)))
        .for_each(|x| entry.add_some(space.index(x)));
    neg.iter()
        .filter(|x| !x.contains(&(SPACE_SIZE as u8 + 1)))
        .for_each(|x| entry.sub_some(space.index(x)));
}

fn base_indices(
    cube: &ColourCube,
    direction: &Direction,
) -> ([[u8; 3]; 2], [[u8; 3]; 2])
{
    let (e, s) = (cube.end, cube.start);
    let (pos, neg) = match direction {
        Direction::Red => (
            [[e.r, e.g, e.b], [e.r, s.g, s.b]],
            [[e.r, s.g, e.b], [e.r, e.g, s.b]],
        ),
        Direction::Blue => (
            [[e.r, e.g, e.b], [s.r, s.g, e.b]],
            [[s.r, e.g, e.b], [e.r, s.g, e.b]],
        ),
        Direction::Green => (
            [[e.r, e.g, e.b], [s.r, e.g, s.b]],
            [[s.r, e.g, e.b], [e.r, e.g, s.b]],
        ),
    };
    (pos, neg)
}

fn shift_indices(
    cube: &ColourCube,
    direction: &Direction,
    shift: u8,
) -> ([[u8; 3]; 2], [[u8; 3]; 2])
{
    let (e, s) = (cube.end, cube.start);
    let (pos, neg) = match direction {
        Direction::Red => (
            //top is e.r
            [[shift, s.g, s.b], [shift, e.g, e.b]],
            [[shift, e.g, s.b], [shift, s.g, e.b]],
        ),
        Direction::Blue => (
            //top is e.b
            [[s.r, s.g, shift], [e.r, e.g, shift]],
            [[e.r, s.g, shift], [s.r, e.g, shift]],
        ),
        Direction::Green => (
            //top is e.g
            [[s.r, shift, s.b], [e.r, shift, e.b]],
            [[e.r, shift, s.b], [s.r, shift, e.b]],
        ),
    };
    (pos, neg)
}

fn all_indices(cube: &ColourCube) -> ([[u8; 3]; 4], [[u8; 3]; 4])
{
    let (e, s) = (cube.end, cube.start);
    let (pos, neg) = (
        [
            [e.r, e.g, e.b],
            [e.r, s.g, s.b], //gb
            [s.r, e.g, s.b], //sb
            [s.r, s.g, e.b], //rg
        ],
        [
            [s.r, s.g, s.b], //rgb
            [s.r, e.g, e.b], //r
            [e.r, s.g, e.b], //g
            [e.r, e.g, s.b], //b
        ],
    );
    (pos, neg)
}

fn variance(cube: &ColourCube, space: &ColourSpace) -> f64
{
    let mut result = ColourEntry::new();
    let (pos, neg) = all_indices(&cube);
    // like combine_some but also takes care of m2 
    pos.iter()
        .filter(|x| !x.contains(&(SPACE_SIZE as u8 + 1)))
        .for_each(|x| result.add_inplace(space.index(x)));
    neg.iter()
        .filter(|x| !x.contains(&(SPACE_SIZE as u8 + 1)))
        .for_each(|x| result.sub_inplace(space.index(x)));

    result.m2 as f64 - result.m.squared() as f64 / result.count as f64
}

fn maximise(
    cube: ColourCube,
    //whole: &ColourEntry,
    space: &ColourSpace,
) -> (ColourCube, ColourCube)
{
    //we'll iterate over these directions and ranges
    let it = [
        (Direction::Red, cube.end.r..cube.start.r),
        (Direction::Green, cube.end.g..cube.start.g),
        (Direction::Blue, cube.end.b..cube.start.b),
    ];

    // some vars for the results
    let mut max = 0.0;
    let mut cut = [[0u8, 0, 0]; 2];

    // get the values for the whole cube
    let mut whole = ColourEntry::new();
    let (pos, neg) = all_indices(&cube);
    combine_some(&pos, &neg, &space, &mut whole);

    for (direction, range) in it {
        let mut base = ColourEntry::new();
        let (pos, neg) = base_indices(&cube, &direction);
        combine_some(&pos, &neg, &space, &mut base);

        for i in range {
            let mut half = base.clone();
            let (pos, neg) = shift_indices(&cube, &direction, i);
            // by switching positive and negative we sum directly into half
            combine_some(&neg, &pos, &space, &mut half);

            // no need to iterate further as this won't be getting smaller!
            if half.count == whole.count {
                continue;
            }
            if half.count == 0 {
                break;
            }

            // idk what else to name this lol
            // also surely this can be optimised ???
            let anti_variance = {
                let other_half = whole.clone().sub(&half);

                half.m.squared() as f64 / half.count as f64
                    + other_half.m.squared() as f64 / other_half.count as f64
            };

            if anti_variance > max {
                max = anti_variance;
                cut = pos;
            }
        }
    }
    (
        ColourCube {
            start: cube.start,
            end: RGB::from(cut[1]),
        },
        ColourCube {
            start: RGB::from(cut[0]),
            end: cube.end,
        },
    )
}

fn process_cuts(
    part: ColourCube,
    queue: &mut Vec<ColourCube>,
    unitv: &mut Vec<ColourCube>,
    space: &ColourSpace,
)
{
    if part.start.sub(&part.end) == UNIT_RGB {
        unitv.push(part)
    } else {
        let v = variance(&part, space);
        queue.push(part);
    }
}

#[allow(dead_code)]
fn compress(palette: &[RGB<u8>]) -> ()
{
    let mut space = ColourSpace::new();
    histogram(palette, &mut space);
    cummulate_vals(&mut space);

    let cube = ColourCube {
        start: RGB::from([SPACE_SIZE as u8 + 1; 3]),
        end: RGB::from([SPACE_SIZE as u8; 3]),
    };

    // unit volume cubes cannot be cut further
    let mut unitv = Vec::new();
    let mut queue = Vec::new();
    queue.push(cube);

    loop {
        match queue.pop() {
            Some(next) => {
                let (part, other_part) = maximise(next, &space);
                process_cuts(part, &mut queue, &mut unitv, &space);
                process_cuts(other_part, &mut queue, &mut unitv, &space);
            }
            None => break,
        }
    }
}
