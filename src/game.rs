
pub enum Flavor {
    KnickKnack,
}

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

pub fn update(pos: &mut [i8; 2], input: Dir) {
    let [dx, dy] = input.to_vec();
    *pos = wrap_pos([pos[0] + dx, pos[1] + dy]);
}
