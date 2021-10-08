// colours have 3 dimesions with weight
// methods to try
// Wu's - very fast, looks better than NQ
// https://doi.org/10.1016/B978-0-08-050754-5.50035-9
//
// BS-ATCQ - very good quality, medium speed
// BKMS - best quality, slowest
// Fast nearest neighbour - looks very promising too

use rgb::RGB;

const COLOURS: usize = 256;
const ROUND_N: usize = 3;
const SPACE_SIZE: usize = (255 >> ROUND_N) + 1;

#[derive(Copy, Clone)]
enum Direction
{
    Red,
    Green,
    Blue,
}

#[derive(Debug)]
struct ColourCube
{
    end: RGB<u8>,
    start: RGB<u8>,
}

#[derive(Copy, Clone)]
struct ColourEntry
{
    pub m: RGB<usize>,
    pub count: usize,
    pub m2: usize,
}

struct ColourSpace
{
    s: [[[ColourEntry; SPACE_SIZE]; SPACE_SIZE]; SPACE_SIZE],
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

impl ColourEntry
{
    fn new() -> Self
    {
        let m = RGB::new(0, 0, 0);
        let (count, m2) = (0, 0);
        ColourEntry { m, count, m2 }
    }

    fn add_some(&mut self, other: &Self)
    {
        //self.m.add_inplace(&other.m);
        self.m += other.m;
        self.count += other.count;
    }

    fn add_inplace(&mut self, other: &Self)
    {
        self.add_some(other);
        self.m2 += other.m2;
    }

    fn sub_some(&mut self, other: &Self)
    {
        self.m -= other.m;
        self.count -= other.count;
    }

    fn sub_inplace(&mut self, other: &Self)
    {
        self.sub_some(other);
        self.m2 -= other.m2;
    }

    fn sub(&self, other: &Self) -> Self
    {
        let mut out = *self;
        out.m -= other.m;
        out.count -= other.count;
        out.m2 -= other.m2;
        out
    }
}

impl<T, U> From<(RGB<U>, T, T)> for ColourEntry
where
    usize: From<T> + From<U>,
{
    fn from(entry_tuple: (RGB<U>, T, T)) -> Self
    {
        let rgb = entry_tuple.0;
        ColourEntry {
            m: RGB {
                r: rgb.r.into(),
                g: rgb.g.into(),
                b: rgb.b.into(),
            },
            count: entry_tuple.1.into(),
            m2: entry_tuple.2.into(),
        }
    }
}

trait Wu
{
    type Size;
    fn round(&self) -> [Self::Size; 3];
    fn squared(&self) -> usize;
}

impl Wu for RGB<u8>
{
    type Size = u8;
    fn round(&self) -> [Self::Size; 3]
    {
        [self.r >> ROUND_N, self.g >> ROUND_N, self.b >> ROUND_N]
    }

    fn squared(&self) -> usize
    {
        self.iter().map(|x| (x as usize).pow(2)).sum()
    }
}

impl Wu for RGB<usize>
{
    type Size = usize;
    fn round(&self) -> [Self::Size; 3]
    {
        [self.r >> ROUND_N, self.g >> ROUND_N, self.b >> ROUND_N]
    }
    fn squared(&self) -> usize
    {
        self.iter().map(|x| x.pow(2)).sum()
    }
}

fn histogram(palette: &[RGB<u8>], space: &mut ColourSpace)
{
    palette
        .iter()
        .map(|pixel| (pixel.round(), pixel))
        .for_each(|(idx, p)| {
            let s = space.index_mut(&idx);
            s.add_inplace(&ColourEntry::from((*p, 1, p.squared())));
        });
}

fn cummulate_vals(space: &mut ColourSpace)
{
    for r in 0..SPACE_SIZE {
        let mut areas = [ColourEntry::new(); SPACE_SIZE];

        for g in 0..SPACE_SIZE {
            let mut line = ColourEntry::new();

            for (b, area) in areas.iter_mut().enumerate() {
                let point = space.s[r][g][b];

                line.add_inplace(&point);
                area.add_inplace(&line);

                space.s[r][g][b] = *area;

                //TODO is this good???
                if r > 0 {
                    let prev = space.s[r - 1][g][b];
                    space.s[r][g][b].add_inplace(&prev);
                }
            }
        }
    }
}

fn combine(
    pos: &[[u8; 3]],
    neg: &[[u8; 3]],
    space: &ColourSpace,
    entry: &mut ColourEntry,
)
{
    pos.iter()
        .filter(|x| !x.contains(&(SPACE_SIZE as u8 + 1)))
        .for_each(|x| entry.add_inplace(space.index(x)));

    neg.iter()
        .filter(|x| !x.contains(&(SPACE_SIZE as u8 + 1)))
        .for_each(|x| entry.sub_inplace(space.index(x)));
}

fn base_indices(
    cube: &ColourCube,
    direction: &Direction,
) -> ([[u8; 3]; 2], [[u8; 3]; 2])
{
    let (e, s) = (cube.end, cube.start);
    let (pos, neg) = match direction {
        Direction::Red => (
            [[s.r, s.g, s.b], [s.r, e.g, e.b]],
            [[s.r, e.g, s.b], [s.r, s.g, e.b]],
        ),
        Direction::Blue => (
            [[s.r, s.g, s.b], [e.r, e.g, s.b]],
            [[e.r, s.g, s.b], [s.r, e.g, s.b]],
        ),
        Direction::Green => (
            [[s.r, s.g, s.b], [e.r, s.g, e.b]],
            [[e.r, s.g, s.b], [s.r, s.g, e.b]],
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

fn variance(cube: &ColourCube, space: &ColourSpace) -> u64
{
    let mut result = ColourEntry::new();
    let (pos, neg) = all_indices(cube);
    // like combine_some but also takes care of m2
    pos.iter()
        .filter(|x| !x.contains(&(SPACE_SIZE as u8 + 1)))
        .for_each(|x| result.add_inplace(space.index(x)));
    neg.iter()
        .filter(|x| !x.contains(&(SPACE_SIZE as u8 + 1)))
        .for_each(|x| result.sub_inplace(space.index(x)));

    (result.m2 - result.m.squared() / result.count) as u64
}

fn variance2(entry: ColourEntry) -> f64
{
    entry.m2 as f64 - entry.m.squared() as f64 / entry.count as f64
}

fn maximise(
    cube: &ColourCube,
    space: &ColourSpace,
) -> Option<(ColourCube, ColourCube)>
{
    let it = [
        (Direction::Red, (cube.start.r + 1) % 34..cube.end.r),
        (Direction::Green, (cube.start.g + 1) % 34..cube.end.g),
        (Direction::Blue, (cube.start.b + 1) % 34..cube.end.b),
    ];
    let mut cut = [[34u8, 0, 0]; 2];
    let (mut v1, mut v2) = (0.0, 0.0);

    let mut whole = ColourEntry::new();
    let (pos, neg) = all_indices(cube);
    combine(&pos, &neg, space, &mut whole);

    if whole.count == 1 {
        return None;
    }
    let mut max = (whole.m2 - whole.m.squared() / whole.count) as f64;

    for (direction, range) in it {
        let mut base = ColourEntry::new();
        let (pos, neg) = base_indices(cube, &direction);
        combine(&pos, &neg, space, &mut base);

        for i in range {
            let mut half = ColourEntry::new();
            let (pos, neg) = shift_indices(cube, &direction, i);
            combine(&pos, &neg, space, &mut half);
            half.sub_inplace(&base);

            if half.count == 0 {
                continue;
            }
            // no need to iterate further as this won't be getting smaller!
            if half.count == whole.count {
                break;
            }

            // surely this can be optimised ???
            let anti_variance = {
                let other_half = whole.clone().sub(&half);

                v1 = variance2(half);
                v2 = variance2(other_half);
                (v1 - v2).abs()
            };
            //println!("{}", anti_variance);

            if anti_variance < max {
                max = anti_variance;
                cut = pos;
            }
        }
    }

    // only cut if the value changed - the else clause is reached if all the
    // points were in a unit section. This should prevent creating an empty cube
    if cut[0][0] != 34 {
        Some((
            ColourCube {
                start: cube.start,
                end: RGB::from(cut[1]),
            },
            ColourCube {
                start: RGB::from(cut[0]),
                end: cube.end,
            },
        ))
    } else {
        None
    }
}

fn process_parts(
    part: ColourCube,
    queue: &mut Vec<(ColourCube, u64)>,
    space: &ColourSpace,
)
{
    // unit volume cubes cannot be cut further
    if part.start.iter().zip(part.end.iter()).all(|(x, y)| {
        x + 1 % (SPACE_SIZE + 2) as u8 == y % (SPACE_SIZE + 2) as u8
    }) {
        //println!("Unit vol");
        queue.insert(0, (part, 0));
    } else {
        let v = variance(&part, space);
        let (Ok(idx) | Err(idx)) =
            queue.binary_search_by(|(_, var)| var.cmp(&v));
        //println!("Put in idx {}, {}", idx, v);
        queue.insert(idx, (part, v));
    }
}

#[allow(dead_code)]
pub fn compress(
    palette: &[RGB<u8>],
) -> (Vec<u8>, [[[u8; SPACE_SIZE]; SPACE_SIZE]; SPACE_SIZE])
{
    let mut space = ColourSpace::new();
    histogram(palette, &mut space);
    cummulate_vals(&mut space);

    let mut queue = Vec::with_capacity(COLOURS);
    let cube = ColourCube {
        start: RGB::from([SPACE_SIZE as u8 + 1; 3]),
        end: RGB::from([SPACE_SIZE as u8 - 1; 3]),
    };
    queue.push((cube, 1));

    while queue.len() < COLOURS {
        match queue.pop() {
            Some((next, v)) => {
                if v == 0 {
                    println!("There are less than {} colours", COLOURS);
                    queue.push((next, 0));
                    break;
                }
                if let Some((part, other_part)) = maximise(&next, &space) {
                    process_parts(part, &mut queue, &space);
                    process_parts(other_part, &mut queue, &space);
                } else {
                    queue.insert(0, (next, 0));
                }
            }
            None => break,
        }
    }

    let mut indices = [[[0u8; SPACE_SIZE]; SPACE_SIZE]; SPACE_SIZE];

    let colours_flat: Vec<u8> = queue
        .iter()
        .enumerate()
        .flat_map(|(i, (cube, _))| {
            mark([cube.start, cube.end], &mut indices, i as u8);
            let mut entry = ColourEntry::new();
            let (pos, neg) = all_indices(cube);
            combine(&pos, &neg, &space, &mut entry);
            entry
                .m
                .iter()
                .map(move |x| (x / entry.count) as u8)
                .collect::<Vec<u8>>()
        })
        .collect();

    (colours_flat, indices)
}

fn mark(p: [RGB<u8>; 2], space: &mut [[[u8; 32]; 32]; 32], i: u8)
{
    let lambda = |x| (x + 1) % 34;
    for r in lambda(p[0].r)..p[1].r + 1 {
        for g in lambda(p[0].g)..p[1].g + 1 {
            for b in lambda(p[0].b)..p[1].b + 1 {
                space[r as usize][g as usize][b as usize] = i;
            }
        }
    }
}
