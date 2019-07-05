
#[derive(Clone, Copy)]
pub enum Command {
    Wait(u64), // (relative) time
    Nav([f32; 2]), // position
}

#[derive(Clone, Copy, PartialEq)]
pub enum Action {
    Wait,
    Move([f32; 2]), // velocity
}

pub type EID = u64;

#[derive(Clone, Copy)]
pub struct UnitState {
    pub pos: [f32; 2],
    pub action: Action,
    pub time: u64,
    pub id: EID,
}

impl UnitState {
    pub fn none() -> Self {
        UnitState {
            pos: [0.0, 0.0],
            action: Action::Wait,
            time: 0,
            id: EID::max_value(),
        }
    }

    pub fn update(self: &mut Self, time: u64) {
        self.pos = self.precise_position(time * 100);
        self.time = time;
    }

    pub fn precise_position(self: &Self, time_milli: u64) -> [f32; 2] {
        let dtime = (time_milli - self.time * 100) as f32 / 100.0;
        let mut result = self.pos;
        if let Action::Move([vx, vy]) = self.action {
            result[0] += vx * dtime;
            result[1] += vy * dtime;
        }
        result
    }

    // fn check_action(self: &Self, action: Action); at some point?
}

pub struct Snapshot(pub Vec<UnitState>);
pub struct Timeline(pub Vec<UnitState>);

impl Timeline {
    pub fn snap(self: &Self, time: u64) -> Snapshot {
        let mut result = Vec::new();
        for &state in &self.0 {
            if state.time > time {
                break;
            }
            while result.len() <= state.id as usize {
                result.push(UnitState::none());
            }
            result[state.id as usize] = state;
        }
        Snapshot(result)
    }
}

pub struct Plan(pub UnitState, pub u64, pub Vec<Command>);

impl UnitState {
    fn act(self: &Self, command: Command) -> (u64, Action) {
        use self::Command::*;
        match command {
            Wait(duration) => {
                (duration, Action::Wait)
            },
            Nav([px, py]) => {
                let dx = px - self.pos[0];
                let dy = py - self.pos[1];
                let dist = (dx * dx + dy * dy).sqrt();
                let MAX_SPEED = 1.0 / 10.0;  // 1 per 10 10ths of a second
                let mut dt = (dist / MAX_SPEED).ceil();
                if dt < 1.0 {
                    dt = 1.0;
                }
                (dt as u64, Action::Move([dx / dt, dy / dt]))
            },
        }
    }
}

pub fn generate_timeline(mut plans: Vec<Plan>) -> Timeline {
    let mut result = Vec::new();
    let mut changed = Vec::new();
    loop {
        let mut next = None;
        for &Plan(state, dt, ref commands) in &plans {
            if state.action != Action::Wait || commands.len() > 0 {
                let improved: bool;
                if let Some(next_t) = next {
                    improved = state.time + dt < next_t;
                } else {
                    improved = true;
                }
                if improved {
                    next = Some(state.time + dt);
                }
            }
        }
        if next.is_none() {
            break;
        }
        let next = next.unwrap();
        for Plan(state, dt, commands) in &mut plans {
            let new_time = state.time + *dt;
            if new_time == next {
                state.update(state.time + *dt);
                let (new_dt, action) = if commands.len() > 0 {
                    state.act(commands.remove(0))
                } else {
                    (0, Action::Wait)
                };
                state.action = action;
                *dt = new_dt;
                changed.push(*state);
            }
        }
        for state in changed.drain(..) {
            result.push(state);
        }
    }
    Timeline(result)
}

