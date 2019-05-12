
#[derive(Clone, Copy, Default)]
pub struct Dir1<T> {
    pub pos: T,
    pub neg: T,
}

#[derive(Clone, Copy, Default)]
pub struct Dir2<T> {
    pub x: Dir1<T>,
    pub y: Dir1<T>,
}

impl Dir1<bool> {
    pub fn dir(self: Self) -> i8 {
        self.pos as i8 - self.neg as i8
    }
}

impl<T> Dir1<T> where T: Clone {
    pub fn write_if_eq<U: PartialEq>(
        self: &mut Self,
        keymap: &Dir1<U>,
        key: &U,
        val: &T,
    ) {
        if *key == keymap.pos {
            self.pos = val.clone();
        }
        if *key == keymap.neg {
            self.neg = val.clone();
        }
    }
}

impl Dir2<bool> {
    pub fn dir(self: Self) -> [i8; 2] {
        [self.x.dir(), self.y.dir()]
    }

    pub fn dir_vec(self: Self) -> [f32; 2] {
        let base = self.dir();
        [base[0] as f32, base[1] as f32]
    }
}

impl<T> Dir2<T> where T: Clone {
    pub fn write_if_eq<U: PartialEq>(
        self: &mut Self,
        keymap: &Dir2<U>,
        key: &U,
        val: &T,
    ) {
        self.x.write_if_eq(&keymap.x, key, val);
        self.y.write_if_eq(&keymap.y, key, val);
    }
}

