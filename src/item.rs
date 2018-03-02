use tcod::colors;
use tcod::input::{self, Event};

use object::*;
use map::Map;
use renderer::render_all;
use {message, Messages, Tcod, MAP_HEIGHT, MAP_WIDTH, PLAYER};

const HEAL_AMOUNT: i32 = 4;
const LIGHTNING_DAMAGE: i32 = 20;
const LIGHTNING_RANGE: i32 = 5;
const CONFUSE_RANGE: i32 = 8;
const CONFUSE_NUM_TURNS: i32 = 10;
const FIREBALL_RADIUS: i32 = 3;
const FIREBALL_DAMAGE: i32 = 12;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Item {
    Heal,
    Lightning,
    Confuse,
    Fireball,
}

enum UseResult {
    UsedUp,
    Cancelled,
}

pub fn pick_item_up(
    object_id: usize,
    objects: &mut Vec<Object>,
    inventory: &mut Vec<Object>,
    messages: &mut Messages,
) {
    if inventory.len() >= 26 {
        message(
            messages,
            format!(
                "Your inventory is full, cannot pick up {}.",
                objects[object_id].name
            ),
            colors::RED,
        );
    } else {
        let item = objects.swap_remove(object_id);
        message(
            messages,
            format!("You picked up a {}!", item.name),
            colors::GREEN,
        );
        inventory.push(item);
    }
}

pub fn drop_item(
    inventory_id: usize,
    inventory: &mut Vec<Object>,
    objects: &mut Vec<Object>,
    messages: &mut Messages,
) {
    let mut item = inventory.remove(inventory_id);
    item.set_pos(objects[PLAYER].x, objects[PLAYER].y);
    message(
        messages,
        format!("You dropped a {}.", item.name),
        colors::YELLOW,
    );
    objects.push(item);
}

pub fn use_item(
    inventory_id: usize,
    inventory: &mut Vec<Object>,
    objects: &mut [Object],
    map: &mut Map,
    tcod: &mut Tcod,
    messages: &mut Messages,
) {
    use self::Item::*;
    if let Some(item) = inventory[inventory_id].item {
        let on_use = match item {
            Heal => cast_heal,
            Lightning => cast_lightning,
            Confuse => cast_confuse,
            Fireball => cast_fireball,
        };
        match on_use(inventory_id, objects, messages, map, tcod) {
            UseResult::UsedUp => {
                inventory.remove(inventory_id);
            }
            UseResult::Cancelled => message(messages, "Cancelled", colors::WHITE),
        }
    } else {
        message(
            messages,
            format!("The {} cannot be used.", inventory[inventory_id].name),
            colors::WHITE,
        );
    }
}

fn cast_heal(
    _inventory_id: usize,
    objects: &mut [Object],
    messages: &mut Messages,
    _map: &mut Map,
    _tcod: &mut Tcod,
) -> UseResult {
    if let Some(fighter) = objects[PLAYER].fighter {
        if fighter.hp == fighter.max_hp {
            message(messages, "You are already at full health.", colors::RED);
            return UseResult::Cancelled;
        }
        message(
            messages,
            "Your wounds start to feel better!",
            colors::LIGHT_VIOLET,
        );
        objects[PLAYER].heal(HEAL_AMOUNT);
        return UseResult::UsedUp;
    }
    UseResult::Cancelled
}

fn cast_lightning(
    _inventory_id: usize,
    objects: &mut [Object],
    messages: &mut Messages,
    _map: &mut Map,
    tcod: &mut Tcod,
) -> UseResult {
    let monster_id = closest_monster(LIGHTNING_RANGE, objects, tcod);
    if let Some(monster_id) = monster_id {
        message(
            messages,
            format!(
                "A lightning bolt strikes the {} with a loud thunder! \
                 The damage is {} hit points.",
                objects[monster_id].name, LIGHTNING_DAMAGE
            ),
            colors::LIGHT_BLUE,
        );
        objects[monster_id].take_damage(LIGHTNING_DAMAGE, messages);
        UseResult::UsedUp
    } else {
        message(messages, "No enemy is close enough to strike.", colors::RED);
        UseResult::Cancelled
    }
}

fn cast_confuse(
    _inventory_id: usize,
    objects: &mut [Object],
    messages: &mut Messages,
    map: &mut Map,
    tcod: &mut Tcod,
) -> UseResult {
    message(
        messages,
        "Left-click an enemy to confuse it, or right-click to cancel.",
        colors::LIGHT_CYAN,
    );
    let monster_id = target_monster(tcod, objects, map, messages, Some(CONFUSE_RANGE as f32));
    if let Some(monster_id) = monster_id {
        let old_ai = objects[monster_id].ai.take().unwrap_or(Ai::Basic);
        objects[monster_id].ai = Some(Ai::Confused {
            previous_ai: Box::new(old_ai),
            num_turns: CONFUSE_NUM_TURNS,
        });
        message(
            messages,
            format!(
                "The eyes of {} look vacant, as he starts to stumble around!",
                objects[monster_id].name
            ),
            colors::LIGHT_GREEN,
        );
        UseResult::UsedUp
    } else {
        message(messages, "No enemy is close enough to strike.", colors::RED);
        UseResult::Cancelled
    }
}

fn cast_fireball(
    _inventory_id: usize,
    objects: &mut [Object],
    messages: &mut Messages,
    map: &mut Map,
    tcod: &mut Tcod,
) -> UseResult {
    message(
        messages,
        "Left-click a target tile for the fireball, or right-click to cancel.",
        colors::LIGHT_CYAN,
    );
    let (x, y) = match target_tile(tcod, objects, map, messages, None) {
        Some(tile_pos) => tile_pos,
        None => return UseResult::Cancelled,
    };
    message(
        messages,
        format!(
            "The fireball explodes, burning everything within {} tiles!",
            FIREBALL_RADIUS
        ),
        colors::ORANGE,
    );

    for obj in objects {
        if obj.distance(x, y) <= FIREBALL_RADIUS as f32 && obj.fighter.is_some() {
            message(
                messages,
                format!(
                    "The {} gets burned for {} hit points.",
                    obj.name, FIREBALL_DAMAGE
                ),
                colors::ORANGE,
            );
            obj.take_damage(FIREBALL_DAMAGE, messages);
        }
    }

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
    map: &mut Map,
    messages: &Messages,
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
        render_all(tcod, objects, map, messages, false);
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
    map: &mut Map,
    messages: &Messages,
    max_range: Option<f32>,
) -> Option<usize> {
    loop {
        match target_tile(tcod, objects, map, messages, max_range) {
            Some((x, y)) => for (id, obj) in objects.iter().enumerate() {
                if obj.pos() == (x, y) && obj.fighter.is_some() && id != PLAYER {
                    return Some(id);
                }
            },
            None => return None,
        }
    }
}
