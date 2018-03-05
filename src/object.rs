use map::*;
use map::Map;

use tcod::console::*;
use tcod::colors::{self, Color};
use tcod::map::Map as FovMap;
use rand::{self, Rng};

use std::cmp;

use item::Item;
use ::*;

#[derive(Debug/*, Serialize, Deserialize*/)]
pub struct Object {
    pub x: i32,
    pub y: i32,
    pub char: char,
    pub color: Color,
    pub name: String,
    pub blocks: bool,
    pub alive: bool,
    pub fighter: Option<Fighter>,
    pub ai: Option<Ai>,
    pub item: Option<Item>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum PlayerAction {
    TookTurn,
    DidntTakeTurn,
    Exit,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Fighter {
    pub max_hp: i32,
    pub hp: i32,
    pub defense: i32,
    pub power: i32,
    pub on_death: DeathCallback,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum DeathCallback {
    Player,
    Monster,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Ai {
    Basic,
    Confused {
        previous_ai: Box<Ai>,
        num_turns: i32,
    },
}

impl Object {
    pub fn new(x: i32, y: i32, char: char, name: &str, color: Color, blocks: bool) -> Self {
        Object {
            x: x,
            y: y,
            char: char,
            color: color,
            name: name.into(),
            blocks: blocks,
            alive: false,
            fighter: None,
            ai: None,
            item: None,
        }
    }

    pub fn draw(&self, con: &mut Console) {
        con.set_default_foreground(self.color);
        con.put_char(self.x, self.y, self.char, BackgroundFlag::None);
    }

    pub fn clear(&self, con: &mut Console) {
        con.put_char(self.x, self.y, ' ', BackgroundFlag::None);
    }

    pub fn pos(&self) -> (i32, i32) {
        (self.x, self.y)
    }

    pub fn set_pos(&mut self, x: i32, y: i32) {
        self.x = x;
        self.y = y;
    }

    pub fn distance_to(&self, other: &Object) -> f32 {
        let dx = other.x - self.x;
        let dy = other.y - self.y;
        ((dx.pow(2) + dy.pow(2)) as f32).sqrt()
    }

    pub fn take_damage(&mut self, damage: i32, messages: &mut Messages) {
        if let Some(fighter) = self.fighter.as_mut() {
            if damage > 0 {
                fighter.hp -= damage;
            }
        }
        if let Some(fighter) = self.fighter {
            if fighter.hp <= 0 {
                self.alive = false;
                fighter.on_death.callback(self, messages);
            }
        }
    }

    pub fn attack(&mut self, target: &mut Object, log: &mut Messages) {
        let damage = self.fighter.map_or(0, |f| f.power) - target.fighter.map_or(0, |f| f.defense);
        if damage > 0 {
            log.add(
                format!(
                    "{} attacks {} for {} hit points.",
                    self.name, target.name, damage
                ),
                colors::RED,
            );
            target.take_damage(damage, log);
        } else {
            log.add(
                format!(
                    "{} attacks {} but it has no effect!",
                    self.name, target.name
                ),
                colors::WHITE,
            );
        }
    }

    pub fn heal(&mut self, amount: i32) {
        if let Some(ref mut fighter) = self.fighter {
            fighter.hp += amount;
            if fighter.hp > fighter.max_hp {
                fighter.hp = fighter.max_hp;
            }
        }
    }

    pub fn distance(&self, x: i32, y: i32) -> f32 {
        (((x - self.x).pow(2) + (y - self.y).pow(2)) as f32).sqrt()
    }
}

impl DeathCallback {
    pub fn callback(self, object: &mut Object, messages: &mut Messages) {
        let callback: fn(&mut Object, &mut Messages) = match self {
            DeathCallback::Player => player_death,
            DeathCallback::Monster => monster_death,
        };
        callback(object, messages);
    }
}

pub fn move_by(id: usize, dx: i32, dy: i32, map: &Map, objects: &mut [Object]) {
    let (x, y) = objects[id].pos();
    if !is_blocked(x + dx, y + dy, map, objects) {
        objects[id].x += dx;
        objects[id].y += dy;
    }
}

pub fn player_move_or_attack(
    id: usize,
    dx: i32,
    dy: i32,
    map: &Map,
    objects: &mut [Object],
    messages: &mut Messages,
) {
    let x = objects[PLAYER].x + dx;
    let y = objects[PLAYER].y + dy;

    let target_id = objects
        .iter()
        .position(|object| object.fighter.is_some() && object.pos() == (x, y));

    match target_id {
        Some(target_id) => {
            let (player, monster) = mut_two(PLAYER, target_id, objects);
            player.attack(monster, messages);
        }
        None => {
            move_by(id, dx, dy, map, objects);
        }
    }
}

pub fn move_towards(id: usize, target_x: i32, target_y: i32, map: &Map, objects: &mut [Object]) {
    let dx = target_x - objects[id].x;
    let dy = target_y - objects[id].y;
    let distance = ((dx.pow(2) + dy.pow(2)) as f32).sqrt();

    let dx = (dx as f32 / distance).round() as i32;
    let dy = (dy as f32 / distance).round() as i32;
    move_by(id, dx, dy, map, objects);
}

pub fn ai_take_turn(
    monster_id: usize,
    objects: &mut [Object],
    game: &mut Game,
    fov_map: &FovMap,
) {
    use self::Ai::*;
    if let Some(ai) = objects[monster_id].ai.take() {
        let new_ai = match ai {
            Basic => ai_basic(monster_id, &game.map, objects, fov_map, &mut game.log),
            Confused {
                previous_ai,
                num_turns,
            } => ai_confused(monster_id, &game.map, objects, &mut game.log, previous_ai, num_turns),
        };
        objects[monster_id].ai = Some(new_ai);
    }
    let (monster_x, monster_y) = objects[monster_id].pos();
    if fov_map.is_in_fov(monster_x, monster_y) {
        if objects[monster_id].distance_to(&objects[PLAYER]) >= 2.0 {
            let (player_x, player_y) = objects[PLAYER].pos();
            move_towards(monster_id, player_x, player_y, &game.map, objects);
        } else if objects[PLAYER].fighter.map_or(false, |f| f.hp > 0) {
            let (monster, player) = mut_two(monster_id, PLAYER, objects);
            monster.attack(player, &mut game.log);
        }
    }
}

pub fn ai_basic(
    monster_id: usize,
    map: &Map,
    objects: &mut [Object],
    fov_map: &FovMap,
    messages: &mut Messages,
) -> Ai {
    let (monster_x, monster_y) = objects[monster_id].pos();
    if fov_map.is_in_fov(monster_x, monster_y) {
        if objects[monster_id].distance_to(&objects[PLAYER]) >= 2.0 {
            let (player_x, player_y) = objects[PLAYER].pos();
            move_towards(monster_id, player_x, player_y, map, objects);
        } else if objects[PLAYER].fighter.map_or(false, |f| f.hp > 0) {
            let (monster, player) = mut_two(monster_id, PLAYER, objects);
            monster.attack(player, messages);
        }
    }
    Ai::Basic
}

pub fn ai_confused(
    monster_id: usize,
    map: &Map,
    objects: &mut [Object],
    log: &mut Messages,
    previous_ai: Box<Ai>,
    num_turns: i32,
) -> Ai {
    if num_turns >= 0 {
        move_by(
            monster_id,
            rand::thread_rng().gen_range(-1, 2),
            rand::thread_rng().gen_range(-1, 2),
            map,
            objects,
        );
        Ai::Confused {
            previous_ai: previous_ai,
            num_turns: num_turns - 1,
        }
    } else {
        log.add(
            format!("The {} is no longer confused!", objects[monster_id].name),
            colors::RED,
        );
        *previous_ai
    }
}

/// Mutably borrow two *separate* elements from the given slice.
/// Panics when the indexes are equal or out of bounds.
fn mut_two<T>(first_index: usize, second_index: usize, items: &mut [T]) -> (&mut T, &mut T) {
    assert!(first_index != second_index);
    let split_at_index = cmp::max(first_index, second_index);
    let (first_slice, second_slice) = items.split_at_mut(split_at_index);
    if first_index < second_index {
        (&mut first_slice[first_index], &mut second_slice[0])
    } else {
        (&mut second_slice[0], &mut first_slice[second_index])
    }
}

fn player_death(player: &mut Object, log: &mut Messages) {
    log.add("You died!", colors::RED);
    player.char = '%';
    player.color = colors::DARK_RED;
}

fn monster_death(monster: &mut Object, log: &mut Messages) {
    log.add(format!("{} is dead!", monster.name), colors::RED);
    monster.char = '%';
    monster.color = colors::DARK_RED;
    monster.blocks = false;
    monster.fighter = None;
    monster.ai = None;
    monster.name = format!("remains of {}", monster.name);
}
