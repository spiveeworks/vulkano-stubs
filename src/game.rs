pub use rand::rngs::ThreadRng as Rng;

#[derive(Clone, Copy)]
pub enum Flavor {
    KnickKnack,
}

#[derive(Clone, Copy)]
pub enum Displacement {
    TL,
    TR,
    M,
    BL,
    BR,
}

const NUM_FLAVORS: u8 = Flavor::KnickKnack as u8 + 1;

pub type World = Vec<([i8; 2], Displacement, Flavor)>;
pub type Inv = Vec<Flavor>;

#[derive(Clone, Copy, PartialEq)]
pub enum Dir {
    Up,
    Down,
    Left,
    Right,
}

impl Dir {
    fn to_vec(self: Dir) -> [i8; 2] {
        use self::Dir::*;
        match self {
            Up    => [ 0, -1],
            Down  => [ 0,  1],
            Left  => [-1,  0],
            Right => [ 1,  0],
        }
    }
}

fn wrap_coord(x: i8) -> i8 {
    (x + 7 + 15) % 15 - 7
}

fn wrap_pos([x, y]: [i8; 2]) -> [i8; 2] {
    [wrap_coord(x), wrap_coord(y)]
}

pub struct Game {
    pub pos: [i8; 2],
    rng: Rng,
    pub world: World,
    pub inv: Inv,
}

impl Game {
    pub fn new() -> Self {
        Game {
            pos: [0, 0],
            rng: rand::thread_rng(),
            world: Vec::new(),
            inv: vec![Flavor::KnickKnack],
        }
    }

    pub fn update(
        self: &mut Game,
        input: Dir,
    ) {
        let [dx, dy] = input.to_vec();
        self.pos = wrap_pos([self.pos[0] + dx, self.pos[1] + dy]);
        {
            use rand::Rng;
            let new_x = self.rng.gen_range(-7, 8);
            let new_y = self.rng.gen_range(-7, 8);
            let flav = match self.rng.gen_range(0, NUM_FLAVORS) {
                0 => Flavor::KnickKnack,
                _ => unreachable!(),
            };
            let disp = match self.rng.gen_range(0, 5) {
                0 => Displacement::TL,
                1 => Displacement::TR,
                2 => Displacement::M,
                3 => Displacement::BL,
                4 => Displacement::BR,
                _ => unreachable!(),
            };
            self.world.push(([new_x, new_y], disp, flav));
        }

        let mut i = 0;
        while i < self.world.len() {
            let (pos, _, flav) = self.world[i];
            if pos == self.pos {
                self.inv.push(flav);
                self.world.remove(i);
            } else {
                i += 1;
            }
        }
    }
}

