use tcod::colors;

use object::*;
use {PLAYER, Messages, message};

const HEAL_AMOUNT: i32 = 4;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Item {
    Heal,
}

enum UseResult {
    UsedUp,
    Cancelled,
}

pub fn pick_item_up(object_id: usize, objects: &mut Vec<Object>,
    inventory: &mut Vec<Object>, messages: &mut Messages) {
    if inventory.len() >= 26 {
        message(messages, format!("Your inventory is full, cannot pick up {}.", objects[object_id].name), colors::RED);
    } else {
        let item = objects.swap_remove(object_id);
        message(messages, format!("You picked up a {}!", item.name), colors::GREEN);
        inventory.push(item);
    }
}

fn use_item(inventory_id: usize, inventory: &mut Vec<Object>, objects: &mut [Object], messages: &mut Messages) {
    use self::Item::*;
    if let Some(item) = inventory[inventory_id].item {
        let on_use = match item {
            Heal => cast_heal,
        };
        match on_use(inventory_id, objects, messages) {
            UseResult::UsedUp => {inventory.remove(inventory_id);},
            UseResult::Cancelled => message(messages, "Cancelled", colors::WHITE),
        }
    } else {
        message(messages, format!("The {} cannot be used.", inventory[inventory_id].name), colors::WHITE);
    }
}

fn cast_heal(_inventory_id: usize, objects: &mut [Object], messages: &mut Messages) -> UseResult {
    if let Some(fighter) = objects[PLAYER].fighter {
        if fighter.hp == fighter.max_hp {
            message(messages, "You are already at full health.", colors::RED);
            return UseResult::Cancelled;
        }
        message(messages, "Your wounds start to feel better!", colors::LIGHT_VIOLET);
        objects[PLAYER].heal(HEAL_AMOUNT);
        return UseResult::UsedUp;
    }
    UseResult::Cancelled
}