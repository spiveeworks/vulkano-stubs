pub use rand::rngs::ThreadRng as Rng;

#[derive(Clone, Copy)]
pub enum PickupFlavor {
    Hunger,
    Nourishment,
}

#[derive(Clone, Copy)]
pub enum Item {
    Hunger(u8),
    Nourishment(u8),
    Health(u8),
    Damage,
}

pub const HUNGER_TIMER: u8 = 10;
pub const NOURISH_TIMER: u8 = 5;
pub const HEALTH_TIMER: u8 = 20;

pub const INV_CAP: usize = 4 * 15;

impl PickupFlavor {
    pub fn pickup(self: Self) -> Item {
        use self::PickupFlavor::*;
        match self {
            Hunger => Item::Hunger(HUNGER_TIMER),
            Nourishment => Item::Nourishment(NOURISH_TIMER),
        }
    }
}

#[derive(Clone, Copy)]
pub enum Displacement {
    TL,
    TR,
    M,
    BL,
    BR,
}

const NUM_FLAVORS: u8 = PickupFlavor::Nourishment as u8 + 1;

// @Readability struct Pickup;?
pub type World = Vec<([i8; 2], Displacement, PickupFlavor)>;

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

#[derive(Default)]
pub struct Game {
    pub pos: [i8; 2],
    rng: Rng,
    pub world: World,
    pub items: Vec<Item>,
    pub counts: [i32; 4],      // could be i8 but whatever
    pub max_counts: [i32; 3],  // could be i8 but whatever
}

impl Game {
    pub fn reset(self: &mut Self) {
        let max_counts = self.max_counts;
        *self = Default::default();
        self.max_counts = max_counts;
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
                0 => PickupFlavor::Hunger,
                1 => PickupFlavor::Nourishment,
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

        let mut pickups = Vec::new();
        {
            let mut i = 0;
            while i < self.world.len() {
                let (pos, _, flav) = self.world[i];
                if pos == self.pos {
                    pickups.push(flav.pickup());
                    self.world.remove(i);
                } else {
                    i += 1;
                }
            }
        }

        fn react_pickups(game: &mut Game, mut pickups: Vec<Item>) {
            while pickups.len() > 0 {
                let pickup = pickups.pop().unwrap();
                let mut matched = false;
                let mut i = game.items.len();
                while !matched && i > 0 {
                    i -= 1;

                    use self::Item::*;
                    match (game.items[i], pickup) {
                        (Nourishment(_), Hunger(_)) |
                        (Hunger(_), Nourishment(_)) => {
                            game.items.remove(i);
                            pickups.push(Health(HEALTH_TIMER));
                            matched = true;
                            break;
                        },
                        (Health(_), Damage) |
                        (Damage, Health(_)) => {
                            game.items.remove(i);
                            pickups.push(Hunger(HUNGER_TIMER));
                            matched = true;
                            break;
                        },
                        (_, _) => (),
                    }
                }
                if !matched && game.items.len() < INV_CAP {
                    game.items.push(pickup);
                }
            }
        }
        react_pickups(self, pickups);

        let mut pickups = Vec::new();
        {
            let mut i = 0;
            while i < self.items.len() {
                use self::Item::*;
                match self.items[i] {
                    Hunger(n) => {
                        if n > 0 {
                            self.items[i] = Hunger(n-1);
                            i += 1;
                        } else {
                            self.items.remove(i);
                            pickups.push(Damage);
                        }
                    },
                    Nourishment(n) => {
                        if n > 0 {
                            self.items[i] = Nourishment(n-1);
                            i += 1;
                        } else {
                            self.items.remove(i);
                        }
                    },
                    Health(n) => {
                        if n > 0 {
                            self.items[i] = Health(n-1);
                            i += 1;
                        } else {
                            self.items.remove(i);
                            pickups.push(Nourishment(NOURISH_TIMER));
                        }
                    },
                    _ => i += 1,
                }
            }
        }

        react_pickups(self, pickups);

        // count everything

        self.counts = [0; 4];

        for &(_, _, pickup) in &self.world {
            self.counts[pickup as usize] += 1;
        }
        for &item in &self.items {
            use self::Item::*;
            let i = match item {
                Hunger(_) => 0,
                Nourishment(_) => 1,
                Health(_) => 2,
                Damage => 3,
            };
            self.counts[i] += 1;
        }

        for i in 0..3 {
            if self.max_counts[i] < self.counts[i] {
                self.max_counts[i] = self.counts[i];
            }
        }
    }
}

