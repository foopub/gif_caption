// colours have 3 dimesions with weight
// methods to try
// Wu's - very fast, looks better than NQ
// https://doi.org/10.1016/B978-0-08-050754-5.50035-9
//
// BS-ATCQ - very good quality, medium speed
// BKMS - best quality, slowest
// Fast nearest neighbour - looks very promising too

use rgb::RGB;

const ROUND_N: usize = 3;
const SPACE_SIZE: usize = 255 >> ROUND_N;
type PSize = u8;

struct ColourSpace
{
    s: [[[ColourEntry; SPACE_SIZE]; SPACE_SIZE]; SPACE_SIZE],
}

#[derive(Copy, Clone)]
struct ColourEntry
{
    pub m: RGB<PSize>,
    pub count: usize,
    pub m2: usize,
}

struct ColourCube
{
    end: RGB<PSize>,
    start: RGB<PSize>,
}

impl ColourEntry
{
    fn new() -> Self
    {
        let m = RGB::new(0 as PSize, 0, 0);
        let (count, m2) = (0, 0);
        ColourEntry { m, count, m2 }
    }
    fn add_inplace(&mut self, other: &Self) -> ()
    {
        self.m.add_inplace(&other.m);
        self.count += other.count;
        self.m2 += other.m2;
    }
    fn sub_inplace(&mut self, other: &Self) -> ()
    {
        self.m.sub_inplace(&other.m);
        self.count -= other.count;
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

impl<T: Into<usize>> From<(&RGB<PSize>, T, T)> for ColourEntry
{
    fn from(entry_tuple: (&RGB<PSize>, T, T)) -> Self
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

    fn index_mut(&mut self, rgb: &RGB<usize>) -> &mut ColourEntry
    {
        &mut self.s[rgb.r][rgb.g][rgb.b]
    }

    fn index(&self, rgb: &(u8, u8, u8)) -> ColourEntry
    {
        self.s[rgb.0 as usize][rgb.1 as usize][rgb.2 as usize]
    }
}

trait Wu
{
    fn dominates(&self, other: Self) -> bool;
    fn round(&self, n_bits: usize) -> RGB<usize>;
    fn squared(&self) -> usize;
    fn add_inplace(&mut self, other: &Self) -> ();
    fn sub_inplace(&mut self, other: &Self) -> ();
}

impl Wu for RGB<u8>
{
    fn dominates(&self, other: Self) -> bool
    {
        (self.r > other.r) & (self.g > other.g) & (self.b > other.b)
    }

    fn round(&self, n_bits: usize) -> RGB<usize>
    {
        //round up the bytes by ignoring the last few bits
        self.iter().map(|x| (x >> n_bits) as usize).collect()
    }

    fn squared(&self) -> usize
    {
        (self.r ^ 2 + self.g ^ 2 + self.b ^ 2) as usize
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
}

/// Iterate over the palette and sum each pixel colour by rgb,
/// P(c) and c^2
fn histogram(palette: &[RGB<u8>], space: &mut ColourSpace) -> ()
{
    // do everything in one iteration over the palette
    palette
        .iter()
        .map(|pixel| (pixel.round(ROUND_N), pixel))
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
                let mut point = space.s[r - 1][g][b];

                line.add_inplace(&point);
                area[b].add_inplace(&line);
                point.add_inplace(&area[b]);
                point.add_inplace(&space.s[r - 1][g][b]);
            }
        }
    }
}

enum Direction
{
    Red,
    Green,
    Blue,
}

fn combine(
    pos: &[(u8, u8, u8)],
    neg: &[(u8, u8, u8)],
    space: &ColourSpace,
    entry: &mut ColourEntry,
) -> ()
{
    //TODO not all cube values are always needed
    pos.iter().for_each(|x| entry.add_inplace(&space.index(x)));
    neg.iter().for_each(|x| entry.sub_inplace(&space.index(x)));
}

fn partial_vals(
    cube: &ColourCube,
    direction: &Direction,
    shift: u8,
) -> ([(u8, u8, u8); 2], [(u8, u8, u8); 2])
{
    let (e, s) = if shift > 0 {
        (cube.start, cube.end)
    } else {
        (cube.end, cube.start)
    };
    let (pos, neg) = match direction {
        Direction::Red => (
            [(e.r + shift, e.g, e.b), (e.r + shift, s.g, s.b)],
            [(e.r + shift, s.g, e.b), (e.r + shift, e.g, s.b)],
        ),
        Direction::Blue => (
            [(e.r, e.g, e.b + shift), (s.r, s.g, e.b + shift)],
            [(s.r, e.g, e.b + shift), (e.r, s.g, e.b + shift)],
        ),
        Direction::Green => (
            [(e.r, e.g + shift, e.b), (s.r, e.g + shift, s.b)],
            [(s.r, e.g + shift, e.b), (e.r, e.g + shift, s.b)],
        ),
    };
    if shift > 0 {
        (neg, pos)
    } else {
        (pos, neg)
    }
}

fn all_vals(cube: &ColourCube) -> ([(u8, u8, u8); 4], [(u8, u8, u8); 4])
{
    let (e, s) = (cube.end, cube.start);
    let (pos, neg) = (
        [
            (e.r, e.g, e.b),
            (e.r, s.g, s.b),
            (s.r, e.g, s.b),
            (s.r, s.g, e.b),
        ],
        [
            (s.r, s.g, s.b),
            (s.r, e.g, e.b),
            (e.r, s.g, e.b),
            (e.r, e.g, s.b),
        ],
    );
    (pos, neg)
}

fn variance(cube: &ColourCube, space: &ColourSpace) -> f64
{
    let mut result = ColourEntry::new();
    let (pos, neg) = all_vals(&cube);
    combine(&pos, &neg, &space, &mut result);
    result.m2 as f64 - result.m.squared() as f64 / result.count as f64
}

fn maximise(
    cube: &ColourCube,
    other_cube: &mut ColourCube,
    //direction: Direction,
    //range: u8,
    whole: &ColourEntry,
    space: &ColourSpace,
) -> (RGB<f64>, RGB<u8>)
{
    let mut max_ = RGB::new(0.0, 0.0, 0.0);
    let mut cut_ = RGB::new(0u8, 0, 0);
    let mut it = [
        (
            Direction::Red,
            cube.end.r - cube.start.r,
            &mut max_.r,
            &mut cut_.r,
        ),
        (
            Direction::Green,
            cube.end.g - cube.start.g,
            &mut max_.g,
            &mut cut_.g,
        ),
        (
            Direction::Blue,
            cube.end.b - cube.start.b,
            &mut max_.b,
            &mut cut_.b,
        ),
    ];
    for (direction, range, max, cut) in it.iter_mut() {
        let mut base = ColourEntry::new();
        let (pos, neg) = partial_vals(&cube, &direction, 0);
        combine(&pos, &neg, &space, &mut base);

        // important to start from 1
        for i in 1..*range {
            let mut half = base.clone();
            let (pos, neg) = partial_vals(&cube, &direction, i);
            combine(&pos, &neg, &space, &mut half);
            if half.count == 0 {
                continue;
            }

            let other_half = whole.clone().sub(&half);
            if other_half.count == 0 {
                continue;
            }

            let temp = half.m.squared() as f64 / half.count as f64
                + other_half.m.squared() as f64 / other_half.count as f64;

            if temp > **max {
                **max = temp;
                **cut = i;
            }
        }
    }
    //easier than dealing with floating point bs for now
    let max = it.iter().max_by_key(|x| (*x.2 * 128.0) as usize).unwrap();
    other_cube.end = cube.end;
    other_cube.start = cube.start;
    (max_, cut_)
}

#[allow(dead_code)]
fn compress(palette: &[RGB<u8>]) -> ()
{
    let mut space = ColourSpace::new();
    histogram(palette, &mut space);
    cummulate_vals(&mut space);
    let mut cube = ColourCube {
        start: RGB::new(0u8, 0, 0),
        end: RGB::new(0u8, 0, 0),
    };
    let mut other_cube = ColourCube {
        start: RGB::new(0u8, 0, 0),
        end: RGB::new(0u8, 0, 0),
    };
    let mut whole = ColourEntry::new();
    let (pos, neg) = all_vals(&cube);
    combine(&pos, &neg, &space, &mut whole);
    maximise(&cube, &mut other_cube, &whole, &space);
}
