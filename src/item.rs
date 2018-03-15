use tcod::colors;
use tcod::input::{self, Event};

use object::*;
use renderer::render_all;
use ::*;

const HEAL_AMOUNT: i32 = 40;
const LIGHTNING_DAMAGE: i32 = 40;
const LIGHTNING_RANGE: i32 = 5;
const CONFUSE_RANGE: i32 = 8;
const CONFUSE_NUM_TURNS: i32 = 10;
const FIREBALL_RADIUS: i32 = 3;
const FIREBALL_DAMAGE: i32 = 25;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Slot {
    LeftHand,
    RightHand,
    Head,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Equipment {
    pub slot: Slot,
    pub equipped: bool,
    pub power_bonus: i32,
    pub defense_bonus: i32,
    pub max_hp_bonus: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Item {
    Heal,
    Lightning,
    Confuse,
    Fireball,
    Sword,
    Shield,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
enum UseResult {
    UsedUp,
    UsedAndKept,
    Cancelled,
}

impl std::fmt::Display for Slot {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            Slot::LeftHand => write!(f, "left hand"),
            Slot::RightHand => write!(f, "right hand"),
            Slot::Head => write!(f, "head"),
        }
    }
}

pub fn pick_item_up(object_id: usize, objects: &mut Vec<Object>, game: &mut Game) {
    if game.inventory.len() >= 26 {
        game.log.add(
            format!(
                "Your inventory is full, cannot pick up {}.",
                objects[object_id].name
            ),
            colors::RED,
        );
    } else {
        let item = objects.swap_remove(object_id);
        game.log
            .add(format!("You picked up a {}!", item.name), colors::GREEN);
        let index = game.inventory.len();
        let slot = item.equipment.map(|e| e.slot);
        game.inventory.push(item);

        // Autoequip
        if let Some(slot) = slot {
            if get_equipment_in_slot(slot, &game.inventory).is_none() {
                game.inventory[index].equip(&mut game.log);
            }
        }
    }
}

pub fn drop_item(inventory_id: usize, objects: &mut Vec<Object>, game: &mut Game) {
    let mut item = game.inventory.remove(inventory_id);
    if item.equipment.is_some() {
        item.unequip(&mut game.log);
    }
    item.set_pos(objects[PLAYER].x, objects[PLAYER].y);
    game.log
        .add(format!("You dropped a {}.", item.name), colors::YELLOW);
    objects.push(item);
}

pub fn use_item(inventory_id: usize, objects: &mut [Object], tcod: &mut Tcod, game: &mut Game) {
    use self::Item::*;
    if let Some(item) = game.inventory[inventory_id].item {
        let on_use = match item {
            Heal => cast_heal,
            Lightning => cast_lightning,
            Confuse => cast_confuse,
            Fireball => cast_fireball,
            Sword => toggle_equipment,
            Shield => toggle_equipment,
        };
        match on_use(inventory_id, objects, game, tcod) {
            UseResult::UsedUp => {
                game.inventory.remove(inventory_id);
            }
            UseResult::UsedAndKept => {}
            UseResult::Cancelled => game.log.add("Cancelled", colors::WHITE),
        }
    } else {
        game.log.add(
            format!("The {} cannot be used.", game.inventory[inventory_id].name),
            colors::WHITE,
        );
    }
}

fn cast_heal(
    _inventory_id: usize,
    objects: &mut [Object],
    game: &mut Game,
    _tcod: &mut Tcod,
) -> UseResult {
    let player = &mut objects[PLAYER];
    if let Some(fighter) = player.fighter {
        if fighter.hp == player.max_hp(game) {
            game.log.add("You are already at full health.", colors::RED);
            return UseResult::Cancelled;
        }
        game.log
            .add("Your wounds start to feel better!", colors::LIGHT_VIOLET);
        player.heal(HEAL_AMOUNT, game);
        return UseResult::UsedUp;
    }
    UseResult::Cancelled
}

fn cast_lightning(
    _inventory_id: usize,
    objects: &mut [Object],
    game: &mut Game,
    tcod: &mut Tcod,
) -> UseResult {
    let monster_id = closest_monster(LIGHTNING_RANGE, objects, tcod);
    if let Some(monster_id) = monster_id {
        game.log.add(
            format!(
                "A lightning bolt strikes the {} with a loud thunder! \
                 The damage is {} hit points.",
                objects[monster_id].name, LIGHTNING_DAMAGE
            ),
            colors::LIGHT_BLUE,
        );
        if let Some(xp) = objects[monster_id].take_damage(LIGHTNING_DAMAGE, &mut game.log) {
            objects[PLAYER].fighter.as_mut().unwrap().xp += xp;
        };
        UseResult::UsedUp
    } else {
        game.log
            .add("No enemy is close enough to strike.", colors::RED);
        UseResult::Cancelled
    }
}

fn cast_confuse(
    _inventory_id: usize,
    objects: &mut [Object],
    game: &mut Game,
    tcod: &mut Tcod,
) -> UseResult {
    game.log.add(
        "Left-click an enemy to confuse it, or right-click to cancel.",
        colors::LIGHT_CYAN,
    );
    let monster_id = target_monster(tcod, objects, game, Some(CONFUSE_RANGE as f32));
    if let Some(monster_id) = monster_id {
        let old_ai = objects[monster_id].ai.take().unwrap_or(Ai::Basic);
        objects[monster_id].ai = Some(Ai::Confused {
            previous_ai: Box::new(old_ai),
            num_turns: CONFUSE_NUM_TURNS,
        });
        game.log.add(
            format!(
                "The eyes of {} look vacant, as he starts to stumble around!",
                objects[monster_id].name
            ),
            colors::LIGHT_GREEN,
        );
        UseResult::UsedUp
    } else {
        game.log
            .add("No enemy is close enough to strike.", colors::RED);
        UseResult::Cancelled
    }
}

fn cast_fireball(
    _inventory_id: usize,
    objects: &mut [Object],
    game: &mut Game,
    tcod: &mut Tcod,
) -> UseResult {
    game.log.add(
        "Left-click a target tile for the fireball, or right-click to cancel.",
        colors::LIGHT_CYAN,
    );
    let (x, y) = match target_tile(tcod, objects, game, None) {
        Some(tile_pos) => tile_pos,
        None => return UseResult::Cancelled,
    };
    game.log.add(
        format!(
            "The fireball explodes, burning everything within {} tiles!",
            FIREBALL_RADIUS
        ),
        colors::ORANGE,
    );

    let mut xp_to_gain = 0;
    for (id, obj) in objects.iter_mut().enumerate() {
        if obj.distance(x, y) <= FIREBALL_RADIUS as f32 && obj.fighter.is_some() {
            game.log.add(
                format!(
                    "The {} gets burned for {} hit points.",
                    obj.name, FIREBALL_DAMAGE
                ),
                colors::ORANGE,
            );
            if let Some(xp) = obj.take_damage(FIREBALL_DAMAGE, &mut game.log) {
                if id != PLAYER {
                    xp_to_gain += xp;
                }
            }
        }
    }

    objects[PLAYER].fighter.as_mut().unwrap().xp += xp_to_gain;

    UseResult::UsedUp
}

fn closest_monster(max_range: i32, objects: &mut [Object], tcod: &Tcod) -> Option<usize> {
    let mut closest_enemy = None;
    let mut closest_dist = (max_range + 1) as f32;
    for (id, object) in objects.iter().enumerate() {
        if (id != PLAYER) && object.fighter.is_some() && object.ai.is_some()
            && tcod.fov.is_in_fov(object.x, object.y)
        {
            let dist = objects[PLAYER].distance_to(object);
            if dist < closest_dist {
                closest_enemy = Some(id);
                closest_dist = dist;
            }
        }
    }
    closest_enemy
}

fn target_tile(
    tcod: &mut Tcod,
    objects: &[Object],
    game: &mut Game,
    max_range: Option<f32>,
) -> Option<(i32, i32)> {
    use tcod::input::KeyCode::Escape;
    loop {
        tcod.root.flush();
        let event = input::check_for_event(input::KEY_PRESS | input::MOUSE).map(|e| e.1);
        let mut key = None;
        match event {
            Some(Event::Mouse(m)) => tcod.mouse = m,
            Some(Event::Key(k)) => key = Some(k),
            None => {}
        }
        render_all(tcod, objects, game, false);
        let (x, y) = (tcod.mouse.cx as i32, tcod.mouse.cy as i32);

        let in_fov = (x < MAP_WIDTH) && (y < MAP_HEIGHT) && tcod.fov.is_in_fov(x, y);
        let in_range = max_range.map_or(true, |range| objects[PLAYER].distance(x, y) <= range);
        if tcod.mouse.lbutton_pressed && in_fov && in_range {
            return Some((x, y));
        }

        let escape = key.map_or(false, |k| k.code == Escape);
        if tcod.mouse.rbutton_pressed || escape {
            return None;
        }
    }
}

fn target_monster(
    tcod: &mut Tcod,
    objects: &[Object],
    game: &mut Game,
    max_range: Option<f32>,
) -> Option<usize> {
    loop {
        match target_tile(tcod, objects, game, max_range) {
            Some((x, y)) => for (id, obj) in objects.iter().enumerate() {
                if obj.pos() == (x, y) && obj.fighter.is_some() && id != PLAYER {
                    return Some(id);
                }
            },
            None => return None,
        }
    }
}

fn toggle_equipment(
    inventory_id: usize,
    _objects: &mut [Object],
    game: &mut Game,
    _tcod: &mut Tcod,
) -> UseResult {
    let equipment = match game.inventory[inventory_id].equipment {
        Some(equipment) => equipment,
        None => return UseResult::Cancelled,
    };
    if equipment.equipped {
        game.inventory[inventory_id].unequip(&mut game.log);
    } else {
        // Unequip old equipment if needed.
        if let Some(old_equipment) = get_equipment_in_slot(equipment.slot, &game.inventory) {
            game.inventory[old_equipment].unequip(&mut game.log);
        }
        game.inventory[inventory_id].equip(&mut game.log);
    }
    UseResult::UsedAndKept
}

fn get_equipment_in_slot(slot: Slot, inventory: &[Object]) -> Option<usize> {
    for (inventory_id, item) in inventory.iter().enumerate() {
        if item.equipment
            .as_ref()
            .map_or(false, |e| e.equipped && e.slot == slot)
        {
            return Some(inventory_id);
        }
    }
    None
}
