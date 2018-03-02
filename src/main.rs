// Following https://tomassedovic.github.io/roguelike-tutorial/part-5-combat.html

extern crate rand;
extern crate tcod;

mod map;
mod object;
mod renderer;
mod item;

use map::*;
use object::*;
use item::{use_item, pick_item_up};
use renderer::MSG_HEIGHT;
use map::{Map, MAP_HEIGHT, MAP_WIDTH};

use tcod::console::*;
use tcod::colors::{self, Color};
use tcod::map::Map as FovMap;
use tcod::input::{self, Event, Key};

const SCREEN_WIDTH: i32 = 80;
const SCREEN_HEIGHT: i32 = 50;
const LIMIT_FPS: i32 = 20;
pub const PLAYER: usize = 0;

type Messages = Vec<(String, Color)>;

fn main() {
    let mut root = Root::initializer()
        .font("arial10x10.png", FontLayout::Tcod)
        .font_type(FontType::Greyscale)
        .size(SCREEN_WIDTH, SCREEN_HEIGHT)
        .title("Rust/libtcod tutorial")
        .init();

    let mut con = Offscreen::new(SCREEN_WIDTH, SCREEN_HEIGHT);
    let mut panel = Offscreen::new(SCREEN_WIDTH, renderer::PANEL_HEIGHT);
    tcod::system::set_fps(LIMIT_FPS);

    let mut player = Object::new(0, 0, '@', "player", colors::WHITE, true);
    player.alive = true;
    player.fighter = Some(Fighter {
        max_hp: 30,
        hp: 30,
        defense: 2,
        power: 5,
        on_death: DeathCallback::Player,
    });

    let mut objects = vec![player];
    let mut map = make_map(&mut objects);

    let mut fov_map = FovMap::new(MAP_WIDTH, MAP_HEIGHT);
    for y in 0..MAP_HEIGHT {
        for x in 0..MAP_WIDTH {
            fov_map.set(
                x,
                y,
                !map[x as usize][y as usize].block_sight,
                !map[x as usize][y as usize].blocked,
            );
        }
    }

    let mut previous_player_position = (-1, -1);

    let mut messages = vec![];
    message(
        &mut messages,
        "Welcome stranger! Prepare to perish in the Tombs of the Ancient Kings.",
        colors::RED,
    );

    let mut mouse = Default::default();
    let mut key = Default::default();

    let mut inventory = vec![];

    // Main loop.
    while !root.window_closed() {
        match input::check_for_event(input::MOUSE | input::KEY_PRESS) {
            Some((_, Event::Mouse(m))) => mouse = m,
            Some((_, Event::Key(k))) => key = k,
            _ => key = Default::default(),
        }

        let fov_recompute = previous_player_position != (objects[PLAYER].x, objects[PLAYER].y);
        renderer::render_all(
            &mut root,
            &mut con,
            &mut panel,
            &objects,
            &mut map,
            &messages,
            &mut fov_map,
            fov_recompute,
            mouse,
        );

        root.flush();

        for object in &objects {
            object.clear(&mut con);
        }

        previous_player_position = (objects[PLAYER].x, objects[PLAYER].y);
        let player_action = handle_keys(key, &mut root, &mut objects, &map, &mut messages, &mut inventory);
        if player_action == PlayerAction::Exit {
            break;
        }

        if objects[PLAYER].alive && player_action != PlayerAction::DidntTakeTurn {
            for id in 0..objects.len() {
                if objects[id].ai.is_some() {
                    ai_take_turn(id, &map, &mut objects, &fov_map, &mut messages);
                }
            }
        }
    }
}

/// Handles keyboard input and returns whether or not
/// the application should exit.
fn handle_keys(
    key: Key,
    root: &mut Root,
    objects: &mut Vec<Object>,
    map: &Map,
    messages: &mut Messages,
    inventory: &mut Vec<Object>
) -> PlayerAction {
    use tcod::input::Key;
    use tcod::input::KeyCode::*;
    use PlayerAction::*;

    let player_alive = objects[PLAYER].alive;
    match (key, player_alive) {
        (Key { code: NumPad8, .. }, true) | (Key { code: Up, .. }, true) => {
            player_move_or_attack(PLAYER, 0, -1, map, objects, messages);
            TookTurn
        }
        (Key { code: NumPad2, .. }, true) | (Key { code: Down, .. }, true) => {
            player_move_or_attack(PLAYER, 0, 1, map, objects, messages);
            TookTurn
        }
        (Key { code: NumPad4, .. }, true) | (Key { code: Left, .. }, true) => {
            player_move_or_attack(PLAYER, -1, 0, map, objects, messages);
            TookTurn
        }
        (Key { code: NumPad6, .. }, true) | (Key { code: Right, .. }, true) => {
            player_move_or_attack(PLAYER, 1, 0, map, objects, messages);
            TookTurn
        }
        (Key { code: NumPad7, .. }, true) => {
            player_move_or_attack(PLAYER, -1, -1, map, objects, messages);
            TookTurn
        }
        (Key { code: NumPad9, .. }, true) => {
            player_move_or_attack(PLAYER, 1, -1, map, objects, messages);
            TookTurn
        }
        (Key { code: NumPad3, .. }, true) => {
            player_move_or_attack(PLAYER, 1, 1, map, objects, messages);
            TookTurn
        }
        (Key { code: NumPad1, .. }, true) => {
            player_move_or_attack(PLAYER, -1, 1, map, objects, messages);
            TookTurn
        }
        (Key { code: NumPad5, .. }, true) => TookTurn,
        (Key { code: End, .. }, true) => TookTurn,
        (Key { printable: 'g', .. }, true) => {
            let item_id = objects.iter().position(|object| {
                object.pos() == objects[PLAYER].pos() && object.item.is_some()
            });
            if let Some(item_id) = item_id {
                pick_item_up(item_id, objects, inventory, messages);
            }
            DidntTakeTurn
        }
        (Key { printable: 'i', .. }, true) => {
            let inventory_index = inventory_menu(
                inventory,
                "Press the key next to an item to use it, or any other to cancel.\n",
                root);
            if let Some(inventory_index) = inventory_index {
                use_item(inventory_index, inventory, objects, messages);
            }
            TookTurn
        }
        (
            Key {
                code: Enter,
                alt: true,
                ..
            },
            _,
        ) => {
            let fullscreen = root.is_fullscreen();
            root.set_fullscreen(!fullscreen);
            DidntTakeTurn
        }
        (Key { code: Escape, .. }, _) => Exit,
        _ => DidntTakeTurn,
    }
}

fn message<T: Into<String>>(messages: &mut Messages, message: T, color: Color) {
    if messages.len() == MSG_HEIGHT {
        messages.remove(0);
    }

    messages.push((message.into(), color));
}

fn inventory_menu(inventory: &[Object], header: &str, root: &mut Root) -> Option<usize> {
    let options = if inventory.len() == 0 {
        vec!["Inventory is empty.".into()]
    } else {
        inventory.iter().map(|item| item.name.clone()).collect()
    };

    menu(header, &options, INVENTORY_WIDTH, root)
}