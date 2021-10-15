// An implementation of Xiaolin Wu's colour quantisation method
// https://doi.org/10.1016/B978-0-08-050754-5.50035-9
// based on the C implementation, found here
// https://gist.github.com/bert/1192520


// !!!!!!!!!!!!!!!!!!!
// TODO A normalisation function would be nice! Right now the colour space is
// filled up linearly but many images only use a limited portion of the full
// spectrum or have an otherwise uneven distribtion.
//
// A test - any image with N distinct colours should remain unchanged for
// quantisation with n_colours >= N
// !!!!!!!!!!!!!!!!!!!

//use std::iter::FromIterator;
use std::ops::Shr;

use rgb::RGB;

//const COLOURS: usize = 8;
const ROUND_N: usize = 3;
const SPACE_SIZE: u8 = (255 >> ROUND_N) + 1;
const SPACE_USIZE: usize = SPACE_SIZE as usize;

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

#[derive(Copy, Clone, Debug)]
struct ColourEntry
{
    pub m: RGB<usize>,
    pub count: usize,
    pub m2: usize,
}

pub struct ColourSpace<T>
{
    s: [[[T; SPACE_USIZE]; SPACE_USIZE]; SPACE_USIZE],
}

trait Wu
{
    fn round(&self) -> Self;
    fn squared(&self) -> usize;
}

impl<T> Wu for RGB<T>
where
    T: Shr<usize, Output = T> + Copy,
    usize: From<T>,
{
    fn round(&self) -> Self
    {
        RGB::from((self.r >> ROUND_N, self.g >> ROUND_N, self.b >> ROUND_N))
    }

    fn squared(&self) -> usize
    {
        self.iter().map(|x| (usize::from(x)).pow(2)).sum()
    }
}

//impl<T, U> From<(T, U)> for ColourCube
//where
//    T: IntoIterator<Item = u8>,
//    U: IntoIterator<Item = u8>,
//{
//    fn from((start, end): (T, U)) -> Self
//    {
//        ColourCube {
//            start: RGB::from_iter(start),
//            end: RGB::from_iter(end),
//        }
//    }
//}
impl From<[RGB<u8>; 2]> for ColourCube
{
    fn from(start_end: [RGB<u8>; 2]) -> Self
    {
        ColourCube {
            start: start_end[0],
            end: start_end[1],
        }
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

impl Default for ColourEntry
{
    fn default() -> Self
    {
        let m = RGB::new(0, 0, 0);
        let (count, m2) = (0, 0);
        ColourEntry { m, count, m2 }
    }
}
impl ColourEntry
{
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

impl<U> ColourSpace<U>
where
    U: Default + Copy,
{
    fn new() -> ColourSpace<U>
    {
        let entry = U::default();
        let s = [[[entry; SPACE_USIZE]; SPACE_USIZE]; SPACE_USIZE];
        ColourSpace { s }
    }

    // T here is usually u8, I am using these functions to easily index
    // into s with indices that aren't usize.
    pub fn index<T>(&self, rgb: RGB<T>) -> &U
    where
        usize: From<T>,
    {
        &self.s[usize::from(rgb.r)][usize::from(rgb.g)][usize::from(rgb.b)]
    }

    fn index_mut<T>(&mut self, rgb: RGB<T>) -> &mut U
    where
        usize: From<T>,
    {
        &mut self.s[usize::from(rgb.r)][usize::from(rgb.g)][usize::from(rgb.b)]
    }
}

impl ColourSpace<ColourEntry>
{
    fn histogram(&mut self, palette: Vec<RGB<u8>>)
    {
        palette.iter().map(|pixel| (pixel.round(), pixel)).for_each(
            |(idx, p)| {
                let s = self.index_mut(idx);
                s.add_inplace(&ColourEntry::from((*p, 1, p.squared())));
            },
        );
    }

    fn cummulate_vals(&mut self)
    {
        for r in 0..SPACE_USIZE {
            let mut areas = [ColourEntry::default(); SPACE_USIZE];

            for g in 0..SPACE_USIZE {
                let mut line = ColourEntry::default();

                for (b, area) in areas.iter_mut().enumerate() {
                    let point = self.s[r][g][b];

                    line.add_inplace(&point);
                    area.add_inplace(&line);

                    self.s[r][g][b] = *area;

                    //TODO is this good???
                    if r > 0 {
                        let prev = self.s[r - 1][g][b];
                        self.s[r][g][b].add_inplace(&prev);
                    }
                }
            }
        }
    }

    fn variance(&self, cube: &ColourCube) -> f64
    {
        let mut entry = ColourEntry::default();
        let (pos, neg) = all_indices(cube);
        pos.iter()
            .filter(|x| !x.as_ref().contains(&(SPACE_SIZE + 1)))
            .for_each(|x| entry.add_inplace(self.index(*x)));
        neg.iter()
            .filter(|x| !x.as_ref().contains(&(SPACE_SIZE + 1)))
            .for_each(|x| entry.sub_inplace(self.index(*x)));
        //.for_each(|x| entry.sub_some(self.index(*x)));
        entry.m2 as f64 - entry.m.squared() as f64 / entry.count as f64
    }

    fn minimise(&self, cube: &ColourCube) -> Option<(ColourCube, ColourCube)>
    {
        let it = [
            (
                Direction::Red,
                (cube.start.r + 1) % (SPACE_SIZE + 2)..cube.end.r,
            ),
            (
                Direction::Green,
                (cube.start.g + 1) % (SPACE_SIZE + 2)..cube.end.g,
            ),
            (
                Direction::Blue,
                (cube.start.b + 1) % (SPACE_SIZE + 2)..cube.end.b,
            ),
        ];
        let mut cut = [RGB::new(0_u8, 0, 0); 2];

        let mut whole = ColourEntry::default();
        let (pos, neg) = all_indices(cube);
        combine_some(&pos, &neg, self, &mut whole);

        if whole.count == 1 {
            return None;
        }

        let mut max = 0.0;

        for (direction, range) in it {
            let mut base = ColourEntry::default();
            let (pos, neg) = base_indices(cube, direction);
            combine_some(&pos, &neg, self, &mut base);

            for i in range {
                let mut half = ColourEntry::default();
                let (pos, neg) = shift_indices(cube, direction, i);
                combine_some(&pos, &neg, self, &mut half);
                half.sub_inplace(&base);

                if half.count == 0 {
                    continue;
                }
                // no need to iterate further as this won't be getting smaller!
                if half.count == whole.count {
                    break;
                }

                // surely this can be optimised ???
                let other_half = whole.clone().sub(&half);
                let variance_diff = {
                    half.m.squared() as f64 / half.count as f64
                        + other_half.m.squared() as f64 / other_half.count as f64
                };

                if variance_diff > max {
                    max = variance_diff;
                    cut = pos;
                }
            }
        }

        // only cut if the value changed - the else clause is reached if all the
        // points were in a unit section. This should prevent creating an empty
        // cube
        if max == 0.0 {
            None
        } else {
            let part = ColourCube::from([cube.start, cut[1]]);
            let other_part = ColourCube::from([cut[0], cube.end]);
            Some((part, other_part))
        }
    }
}

fn combine_some(
    pos: &[RGB<u8>],
    neg: &[RGB<u8>],
    space: &ColourSpace<ColourEntry>,
    entry: &mut ColourEntry,
)
{
    pos.iter()
        .filter(|x| !x.as_ref().contains(&(SPACE_SIZE + 1)))
        .for_each(|x| entry.add_some(space.index(*x)));

    neg.iter()
        .filter(|x| !x.as_ref().contains(&(SPACE_SIZE + 1)))
        .for_each(|x| entry.sub_some(space.index(*x)));
}

fn base_indices(
    cube: &ColourCube,
    direction: Direction,
) -> ([RGB<u8>; 2], [RGB<u8>; 2])
{
    let (e, s) = (cube.end, cube.start);
    let (pos, neg) = match direction {
        Direction::Red => (
            [RGB::from([s.r, s.g, s.b]), RGB::from([s.r, e.g, e.b])],
            [RGB::from([s.r, e.g, s.b]), RGB::from([s.r, s.g, e.b])],
        ),
        Direction::Blue => (
            [RGB::from([s.r, s.g, s.b]), RGB::from([e.r, e.g, s.b])],
            [RGB::from([e.r, s.g, s.b]), RGB::from([s.r, e.g, s.b])],
        ),
        Direction::Green => (
            [RGB::from([s.r, s.g, s.b]), RGB::from([e.r, s.g, e.b])],
            [RGB::from([e.r, s.g, s.b]), RGB::from([s.r, s.g, e.b])],
        ),
    };
    (pos, neg)
}

fn shift_indices(
    cube: &ColourCube,
    direction: Direction,
    shift: u8,
) -> ([RGB<u8>; 2], [RGB<u8>; 2])
{
    let (e, s) = (cube.end, cube.start);
    let (pos, neg) = match direction {
        Direction::Red => (
            //top is e.r
            [RGB::from([shift, s.g, s.b]), RGB::from([shift, e.g, e.b])],
            [RGB::from([shift, e.g, s.b]), RGB::from([shift, s.g, e.b])],
        ),
        Direction::Blue => (
            //top is e.b
            [RGB::from([s.r, s.g, shift]), RGB::from([e.r, e.g, shift])],
            [RGB::from([e.r, s.g, shift]), RGB::from([s.r, e.g, shift])],
        ),
        Direction::Green => (
            //top is e.g
            [RGB::from([s.r, shift, s.b]), RGB::from([e.r, shift, e.b])],
            [RGB::from([e.r, shift, s.b]), RGB::from([s.r, shift, e.b])],
        ),
    };
    (pos, neg)
}

fn all_indices(cube: &ColourCube) -> ([RGB<u8>; 4], [RGB<u8>; 4])
{
    let (e, s) = (cube.end, cube.start);
    let (pos, neg) = (
        [
            RGB::from([e.r, e.g, e.b]),
            RGB::from([e.r, s.g, s.b]), //gb
            RGB::from([s.r, e.g, s.b]), //sb
            RGB::from([s.r, s.g, e.b]), //rg
        ],
        [
            RGB::from([s.r, s.g, s.b]), //rgb
            RGB::from([s.r, e.g, e.b]), //r
            RGB::from([e.r, s.g, e.b]), //g
            RGB::from([e.r, e.g, s.b]), //b
        ],
    );
    (pos, neg)
}

fn process_part(
    variance: f64,
    cube: ColourCube,
    queue: &mut Vec<(ColourCube, usize)>,
)
{
    if cube
        .start
        .iter()
        .zip(cube.end.iter())
        .all(|(x, y)| x + 1 % (SPACE_SIZE + 2) == y % (SPACE_SIZE + 2))
    {
        queue.insert(0, (cube, 0));
        return;
    }
    let (Ok(idx) | Err(idx)) =
        queue.binary_search_by(|(_, var)| var.cmp(&(variance as usize)));
    queue.insert(idx, (cube, variance as usize));
}

fn mark(p: [RGB<u8>; 2], space: &mut [[[u8; 32]; 32]; 32], i: u8)
{
    let lambda = |x| (x + 1) % (SPACE_SIZE + 2);
    for r in lambda(p[0].r)..=p[1].r {
        for g in lambda(p[0].g)..=p[1].g {
            for b in lambda(p[0].b)..=p[1].b {
                space[r as usize][g as usize][b as usize] = i;
            }
        }
    }
}

#[allow(dead_code)]
pub fn compress(
    palette: Vec<RGB<u8>>,
    n_colours: usize,
) -> (Vec<u8>, ColourSpace<u8>)
{
    let mut space = ColourSpace::new();
    space.histogram(palette);
    space.cummulate_vals();

    let cube = ColourCube {
        start: RGB::from([SPACE_SIZE + 1; 3]),
        end: RGB::from([SPACE_SIZE - 1; 3]),
    };

    let mut queue = Vec::with_capacity(n_colours);
    queue.push((cube, 1));
    // This entire loop adds at most one element to the queue per
    // iter so it won't need to reallocate.
    while let Some((next, var)) = queue.pop() {
        if var == 0 {
            break;
        }
        if let Some((part, other_part)) = space.minimise(&next) {
            // Passing variance first to avoid move problems
            process_part(space.variance(&part), part, &mut queue);
            process_part(space.variance(&other_part), other_part, &mut queue);
        } else {
            queue.insert(0, (next, 0));
        }
        if queue.len() == n_colours {
            break;
        }
    }

    let mut indices = ColourSpace::new();

    let colours_flat: Vec<u8> = queue
        .iter()
        .enumerate()
        .flat_map(|(i, (cube, _))| {
            mark([cube.start, cube.end], &mut indices.s, i as u8);
            let mut entry = ColourEntry::default();
            let (pos, neg) = all_indices(cube);
            combine_some(&pos, &neg, &space, &mut entry);
            entry
                .m
                .iter()
                .map(move |x| (x / entry.count) as u8)
                .collect::<Vec<u8>>()
        })
        .collect();

    (colours_flat, indices)
}
