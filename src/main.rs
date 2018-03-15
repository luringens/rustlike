// Following https://tomassedovic.github.io/roguelike-tutorial/part-5-combat.html

extern crate rand;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate tcod;

mod map;
mod object;
mod renderer;
mod item;
mod fov;

use map::*;
use object::*;
use item::*;
use renderer::{menu, MSG_HEIGHT};
use map::{Map, MAP_HEIGHT, MAP_WIDTH};
use fov::Fov;

use tcod::console::*;
use tcod::colors::{self, Color};
use tcod::input::{self, Event, Key, Mouse};

const SCREEN_WIDTH: i32 = 80;
const SCREEN_HEIGHT: i32 = 50;
const LIMIT_FPS: i32 = 20;
const PLAYER: usize = 0;
const LEVEL_UP_BASE: i32 = 200;
const LEVEL_UP_FACTOR: i32 = 150;
const LEVEL_SCREEN_WIDTH: i32 = 40;
const CHARACTER_SCREEN_WIDTH: i32 = 30;

type Messages = Vec<(String, Color)>;

//#[derive(Serialize, Deserialize)]
pub struct Tcod {
    root: Root,
    con: Offscreen,
    panel: Offscreen,
    fov: Fov,
    mouse: Mouse,
}

//#[derive(Serialize, Deserialize)]
pub struct Game {
    map: Map,
    log: Messages,
    inventory: Vec<Object>,
    dungeon_level: u32,
    player_level: i32,
}

fn main() {
    let root = Root::initializer()
        .font("arial10x10.png", FontLayout::Tcod)
        .font_type(FontType::Greyscale)
        .size(SCREEN_WIDTH, SCREEN_HEIGHT)
        .title("Rust/libtcod tutorial")
        .init();
    tcod::system::set_fps(LIMIT_FPS);

    let mut tcod = Tcod {
        root: root,
        con: Offscreen::new(SCREEN_WIDTH, SCREEN_HEIGHT),
        panel: Offscreen::new(SCREEN_WIDTH, renderer::PANEL_HEIGHT),
        fov: Fov::new(),
        mouse: Default::default(),
    };

    main_menu(&mut tcod);
}

fn main_menu(tcod: &mut Tcod) {
    let img = tcod::image::Image::from_file("menu_background.png")
        .ok()
        .expect("Backgrounds image not found");
    while !tcod.root.window_closed() {
        tcod::image::blit_2x(&img, (0, 0), (-1, -1), &mut tcod.root, (0, 0));

        tcod.root.set_default_foreground(colors::LIGHT_YELLOW);
        tcod.root.print_ex(
            SCREEN_WIDTH / 2,
            SCREEN_HEIGHT / 2 - 4,
            BackgroundFlag::None,
            TextAlignment::Center,
            "TOMBS OF THE ANCIENT KINGS",
        );
        tcod.root.print_ex(
            SCREEN_WIDTH / 2,
            SCREEN_HEIGHT - 2,
            BackgroundFlag::None,
            TextAlignment::Center,
            "Luringen",
        );

        let choices = &["Play a new game", "Continue last game", "Quit"];
        let choice = menu("", choices, 24, &mut tcod.root);
        match choice {
            Some(0) => {
                let (mut objects, mut game) = new_game(tcod);
                play_game(&mut objects, &mut game, tcod);
            }
            Some(2) => break,
            _ => {}
        }
    }
}

fn play_game(objects: &mut Vec<Object>, game: &mut Game, tcod: &mut Tcod) {
    let mut previous_player_position = (-1, -1);
    let mut key = Default::default();

    // Main loop.
    while !tcod.root.window_closed() {
        match input::check_for_event(input::MOUSE | input::KEY_PRESS) {
            Some((_, Event::Mouse(m))) => tcod.mouse = m,
            Some((_, Event::Key(k))) => key = k,
            _ => key = Default::default(),
        }

        let fov_recompute = previous_player_position != (objects[PLAYER].x, objects[PLAYER].y);
        renderer::render_all(tcod, &objects, game, fov_recompute);

        tcod.root.flush();

        level_up(objects, game, tcod);

        for object in objects.iter_mut() {
            object.clear(&mut tcod.con);
        }

        previous_player_position = (objects[PLAYER].x, objects[PLAYER].y);
        let player_action = handle_keys(key, tcod, objects, game);
        if player_action == PlayerAction::Exit {
            break;
        }

        if objects[PLAYER].alive && player_action != PlayerAction::DidntTakeTurn {
            for id in 0..objects.len() {
                if objects[id].ai.is_some() {
                    ai_take_turn(id, objects, game, &tcod.fov);
                }
            }
        }
    }
}

fn new_game(tcod: &mut Tcod) -> (Vec<Object>, Game) {
    let mut player = Object::new(0, 0, '@', "player", colors::WHITE, true);
    player.alive = true;
    player.fighter = Some(Fighter {
        base_max_hp: 100,
        hp: 100,
        base_defense: 1,
        base_power: 2,
        on_death: DeathCallback::Player,
        xp: 0,
    });

    let mut objects = vec![player];
    let mut game = Game {
        map: make_map(&mut objects, 1),
        log: vec![],
        inventory: vec![],
        dungeon_level: 1,
        player_level: 1,
    };
    
    let mut dagger = Object::new(0, 0, '-', "dagger", colors::SKY, false);
    dagger.item = Some(Item::Sword);
    dagger.equipment = Some(Equipment {
        equipped: true,
        slot: Slot::LeftHand,
        max_hp_bonus: 0,
        defense_bonus: 0,
        power_bonus: 2,
    });
    game.inventory.push(dagger);

    initialize_fov(&game.map, tcod);

    game.log.add(
        "Welcome stranger! Prepare to perish in the Tombs of the Ancient Kings.",
        colors::RED,
    );

    (objects, game)
}

fn initialize_fov(map: &Map, tcod: &mut Tcod) {
    tcod.fov = Fov::from_map(map);
    tcod.con.clear(); // Clear out previous FOV.
}

/// Handles keyboard input and returns whether or not
/// the application should exit.
fn handle_keys(
    key: Key,
    tcod: &mut Tcod,
    objects: &mut Vec<Object>,
    game: &mut Game,
) -> PlayerAction {
    use tcod::input::Key;
    use tcod::input::KeyCode::*;
    use PlayerAction::*;

    let player_alive = objects[PLAYER].alive;
    match (key, player_alive) {
        (Key { code: NumPad8, .. }, true) | (Key { code: Up, .. }, true) => {
            player_move_or_attack(PLAYER, 0, -1, objects, game);
            TookTurn
        }
        (Key { code: NumPad2, .. }, true) | (Key { code: Down, .. }, true) => {
            player_move_or_attack(PLAYER, 0, 1, objects, game);
            TookTurn
        }
        (Key { code: NumPad4, .. }, true) | (Key { code: Left, .. }, true) => {
            player_move_or_attack(PLAYER, -1, 0, objects, game);
            TookTurn
        }
        (Key { code: NumPad6, .. }, true) | (Key { code: Right, .. }, true) => {
            player_move_or_attack(PLAYER, 1, 0, objects, game);
            TookTurn
        }
        (Key { code: NumPad7, .. }, true) => {
            player_move_or_attack(PLAYER, -1, -1, objects, game);
            TookTurn
        }
        (Key { code: NumPad9, .. }, true) => {
            player_move_or_attack(PLAYER, 1, -1, objects, game);
            TookTurn
        }
        (Key { code: NumPad3, .. }, true) => {
            player_move_or_attack(PLAYER, 1, 1, objects, game);
            TookTurn
        }
        (Key { code: NumPad1, .. }, true) => {
            player_move_or_attack(PLAYER, -1, 1, objects, game);
            TookTurn
        }
        (Key { code: NumPad5, .. }, true) => TookTurn,
        (Key { code: End, .. }, true) => TookTurn,
        (Key { printable: 'g', .. }, true) => {
            let item_id = objects
                .iter()
                .position(|object| object.pos() == objects[PLAYER].pos() && object.item.is_some());
            if let Some(item_id) = item_id {
                pick_item_up(item_id, objects, game);
            }
            DidntTakeTurn
        }
        (Key { printable: 'i', .. }, true) => {
            let inventory_index = inventory_menu(
                &mut game.inventory,
                "Press the key next to an item to use it, or any other to cancel.\n",
                &mut tcod.root,
            );
            if let Some(inventory_index) = inventory_index {
                use_item(inventory_index, objects, tcod, game);
            }
            DidntTakeTurn
        }
        (Key { printable: 'd', .. }, true) => {
            let inventory_index = inventory_menu(
                &mut game.inventory,
                "Press the key next to an item to drop it, or any other to cancel.\n",
                &mut tcod.root,
            );
            if let Some(inventory_index) = inventory_index {
                drop_item(inventory_index, objects, game);
            }
            DidntTakeTurn
        }
        (Key { printable: '<', .. }, true) => {
            let player_on_stairs = objects
                .iter()
                .any(|object| object.pos() == objects[PLAYER].pos() && object.name == "stairs");
            if player_on_stairs {
                next_level(game, objects, tcod);
            }
            DidntTakeTurn
        }
        (Key { printable: 'c', .. }, true) => {
            let player = &objects[PLAYER];
            let level = game.player_level;
            let level_up_xp = LEVEL_UP_BASE + level * LEVEL_UP_FACTOR;
            if let Some(fighter) = player.fighter.as_ref() {
                let msg = format!(
                    "Character information
Level: {}
Experience: {}
Experience to level up: {}

Maximum HP: {}
Attack: {}
Defense: {}",
                    level, fighter.xp, level_up_xp, player.max_hp(game), player.power(game), player.defense(game)
                );
                msgbox(&msg, CHARACTER_SCREEN_WIDTH, &mut tcod.root);
            }
            DidntTakeTurn
        }
        (
            Key {
                code: Enter,
                alt: true,
                ..
            },
            _,
        ) => {
            let fullscreen = tcod.root.is_fullscreen();
            tcod.root.set_fullscreen(!fullscreen);
            DidntTakeTurn
        }
        (Key { code: Escape, .. }, _) => Exit,
        _ => DidntTakeTurn,
    }
}

fn inventory_menu(inventory: &[Object], header: &str, root: &mut Root) -> Option<usize> {
    let options = if inventory.len() == 0 {
        vec!["Inventory is empty.".into()]
    } else {
        inventory
            .iter()
            .map(|item| match item.equipment {
                Some(equipment) if equipment.equipped => {
                    format!("{} (on {}", item.name, equipment.slot)
                }
                _ => item.name.clone(),
            })
            .collect()
    };

    menu(header, &options, renderer::INVENTORY_WIDTH, root)
}

trait MessageLog {
    fn add<T: Into<String>>(&mut self, message: T, color: Color);
}

impl MessageLog for Vec<(String, Color)> {
    fn add<T: Into<String>>(&mut self, message: T, color: Color) {
        if self.len() == MSG_HEIGHT {
            self.remove(0);
        }
        self.push((message.into(), color));
    }
}

fn next_level(game: &mut Game, objects: &mut Vec<Object>, tcod: &mut Tcod) {
    game.log.add(
        "You take a moment to rest, and recover your strength.",
        colors::VIOLET,
    );
    let heal_hp = objects[PLAYER].max_hp(game) / 2;
    objects[PLAYER].heal(heal_hp, game);

    game.log.add(
        "After a rare moment of peace, you descend deeper into the heart of the dungeon...",
        colors::RED,
    );
    game.dungeon_level += 1;
    game.map = make_map(objects, game.dungeon_level);
    initialize_fov(&game.map, tcod);
}

fn level_up(objects: &mut [Object], game: &mut Game, tcod: &mut Tcod) {
    let player = &mut objects[PLAYER];
    let level_up_xp = LEVEL_UP_BASE + game.player_level * LEVEL_UP_FACTOR;
    if player.fighter.as_ref().map_or(0, |f| f.xp) >= level_up_xp {
        game.player_level += 1;
        game.log.add(
            format!(
                "Your battle skills grow stronger! You reached level {}!",
                game.player_level
            ),
            colors::YELLOW,
        );

        let fighter = player.fighter.as_mut().unwrap();
        let mut choice = None;
        while choice.is_none() {
            choice = menu(
                "Level up! Choose a stat to raise:\n",
                &[
                    format!("Constitution (+20 HP, from {})", fighter.base_max_hp),
                    format!("Strength (+1 attack, from {})", fighter.base_power),
                    format!("Agility (+1 defense, from {})", fighter.base_defense),
                ],
                LEVEL_SCREEN_WIDTH,
                &mut tcod.root,
            );
        }
        fighter.xp -= level_up_xp;
        match choice.unwrap() {
            0 => {
                fighter.base_max_hp += 20;
                fighter.hp += 20;
            } 
            1 => {
                fighter.base_power += 1;
            }
            2 => {
                fighter.base_defense += 1;
            }
            _ => unreachable!(),
        }
    }
}

fn msgbox(text: &str, width: i32, root: &mut Root) {
    let options: &[&str] = &[];
    menu(text, options, width, root);
}
