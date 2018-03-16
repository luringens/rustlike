// Following https://tomassedovic.github.io/roguelike-tutorial/part-5-combat.html

extern crate rand;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate glutin_window;
extern crate graphics;
extern crate opengl_graphics;
extern crate piston;

mod map;
mod object;
mod renderer;
mod item;
mod fov;
mod console;

use map::*;
use object::*;
use item::*;
use renderer::{menu, MSG_HEIGHT};
use map::{Map, MAP_HEIGHT, MAP_WIDTH};
use fov::Fov;
use console::Console;
use piston::input::*;

const SCREEN_WIDTH: i32 = 80;
const SCREEN_HEIGHT: i32 = 50;
const LIMIT_FPS: i32 = 20;
const PLAYER: usize = 0;
const LEVEL_UP_BASE: i32 = 200;
const LEVEL_UP_FACTOR: i32 = 150;
const LEVEL_SCREEN_WIDTH: i32 = 40;
const CHARACTER_SCREEN_WIDTH: i32 = 30;

type Messages = Vec<(String, [f32; 4])>;

//#[derive(Serialize, Deserialize)]
pub struct Ui {
    root: Console,
    con: Console,
    panel: Console,
    fov: Fov,
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
    let root = Console::initializer()
        .font("arial10x10.png", FontLayout::Ui)
        .font_type(FontType::Greyscale)
        .size(SCREEN_WIDTH, SCREEN_HEIGHT)
        .title("Rust/libui tutorial")
        .init();
    ui::system::set_fps(LIMIT_FPS);

    let mut ui = Ui {
        root: Console::new(SCREEN_WIDTH, SCREEN_HEIGHT),
        con: Console::new(SCREEN_WIDTH, SCREEN_HEIGHT),
        panel: Console::new(SCREEN_WIDTH, renderer::PANEL_HEIGHT),
        fov: Fov::new(),
    };

    main_menu(&mut ui);
}

fn main_menu(ui: &mut Ui) {
    while !ui.root.window_closed() {
        ui.root.set_default_foreground(colors::LIGHT_YELLOW);
        ui.root.print_centered(
            SCREEN_WIDTH / 2,
            SCREEN_HEIGHT / 2 - 4,
            "TOMBS OF THE ANCIENT KINGS",
        );
        ui.root.print_centered(
            SCREEN_WIDTH / 2,
            SCREEN_HEIGHT - 2,
            "Luringen",
        );

        let choices = &["Play a new game", "Continue last game", "Quit"];
        let choice = menu("", choices, 24, &mut ui.root);
        match choice {
            Some(0) => {
                let (mut objects, mut game) = new_game(ui);
                play_game(&mut objects, &mut game, ui);
            }
            Some(2) => break,
            _ => {}
        }
    }
}

fn play_game(objects: &mut Vec<Object>, game: &mut Game, ui: &mut Ui) {
    let mut previous_player_position = (-1, -1);
    let mut key = Default::default();

    // Main loop.
    while !ui.root.window_closed() {
        match input::check_for_event(input::MOUSE | input::KEY_PRESS) {
            Some((_, Event::Mouse(m))) => ui.mouse = m,
            Some((_, Event::Key(k))) => key = k,
            _ => key = Default::default(),
        }

        let fov_recompute = previous_player_position != (objects[PLAYER].x, objects[PLAYER].y);
        renderer::render_all(ui, &objects, game, fov_recompute);

        ui.root.flush();

        level_up(objects, game, ui);

        for object in objects.iter_mut() {
            object.clear(&mut ui.con);
        }

        previous_player_position = (objects[PLAYER].x, objects[PLAYER].y);
        let player_action = handle_keys(key, ui, objects, game);
        if player_action == PlayerAction::Exit {
            break;
        }

        if objects[PLAYER].alive && player_action != PlayerAction::DidntTakeTurn {
            for id in 0..objects.len() {
                if objects[id].ai.is_some() {
                    ai_take_turn(id, objects, game, &ui.fov);
                }
            }
        }
    }
}

fn new_game(ui: &mut Ui) -> (Vec<Object>, Game) {
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

    initialize_fov(&game.map, ui);

    game.log.add(
        "Welcome stranger! Prepare to perish in the Tombs of the Ancient Kings.",
        colors::RED,
    );

    (objects, game)
}

fn initialize_fov(map: &Map, ui: &mut Ui) {
    ui.fov = Fov::from_map(map);
    ui.con.clear(); // Clear out previous FOV.
}

/// Handles keyboard input and returns whether or not
/// the application should exit.
fn handle_keys(
    key: Key,
    ui: &mut Ui,
    objects: &mut Vec<Object>,
    game: &mut Game,
    btn: &Button,
) -> PlayerAction {
    use PlayerAction::*;

    let player_alive = objects[PLAYER].alive;
    match (btn, player_alive) {
        (&Button::Keyboard(Key::NumPad8), true) | (&Button::Keyboard(Key::Up), true) => {
            player_move_or_attack(PLAYER, 0, -1, objects, game);
            TookTurn
        }
        (&Button::Keyboard(Key::NumPad2), true) | (&Button::Keyboard(Key::Down), true) => {
            player_move_or_attack(PLAYER, 0, 1, objects, game);
            TookTurn
        }
        (&Button::Keyboard(Key::NumPad4), true) | (&Button::Keyboard(Key::Left), true) => {
            player_move_or_attack(PLAYER, -1, 0, objects, game);
            TookTurn
        }
        (&Button::Keyboard(Key::NumPad6), true) | (&Button::Keyboard(Key::Right), true) => {
            player_move_or_attack(PLAYER, 1, 0, objects, game);
            TookTurn
        }
        (&Button::Keyboard(Key::NumPad7), true) => {
            player_move_or_attack(PLAYER, -1, -1, objects, game);
            TookTurn
        }
        (&Button::Keyboard(Key::NumPad9), true) => {
            player_move_or_attack(PLAYER, 1, -1, objects, game);
            TookTurn
        }
        (&Button::Keyboard(Key::NumPad3), true) => {
            player_move_or_attack(PLAYER, 1, 1, objects, game);
            TookTurn
        }
        (&Button::Keyboard(Key::NumPad1), true) => {
            player_move_or_attack(PLAYER, -1, 1, objects, game);
            TookTurn
        }
        (&Button::Keyboard(Key::NumPad5), true) => TookTurn,
        (&Button::Keyboard(Key::End), true) => TookTurn,
        (&Button::Keyboard(Key::G), true) => {
            let item_id = objects
                .iter()
                .position(|object| object.pos() == objects[PLAYER].pos() && object.item.is_some());
            if let Some(item_id) = item_id {
                pick_item_up(item_id, objects, game);
            }
            DidntTakeTurn
        }
        (&Button::Keyboard(Key::I), true) => {
            let inventory_index = inventory_menu(
                &mut game.inventory,
                "Press the key next to an item to use it, or any other to cancel.\n",
                &mut ui.root,
            );
            if let Some(inventory_index) = inventory_index {
                use_item(inventory_index, objects, ui, game);
            }
            DidntTakeTurn
        }
        (&Button::Keyboard(Key::D), true) => {
            let inventory_index = inventory_menu(
                &mut game.inventory,
                "Press the key next to an item to drop it, or any other to cancel.\n",
                &mut ui.root,
            );
            if let Some(inventory_index) = inventory_index {
                drop_item(inventory_index, objects, game);
            }
            DidntTakeTurn
        }
        (&Button::Keyboard(Key::Less), true) => {
            let player_on_stairs = objects
                .iter()
                .any(|object| object.pos() == objects[PLAYER].pos() && object.name == "stairs");
            if player_on_stairs {
                next_level(game, objects, ui);
            }
            DidntTakeTurn
        }
        (&Button::Keyboard(Key::C), true) => {
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
                    level,
                    fighter.xp,
                    level_up_xp,
                    player.max_hp(game),
                    player.power(game),
                    player.defense(game)
                );
                msgbox(&msg, CHARACTER_SCREEN_WIDTH, &mut ui.root);
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
            let fullscreen = ui.root.is_fullscreen();
            ui.root.set_fullscreen(!fullscreen);
            DidntTakeTurn
        }
        (Key { code: Escape, .. }, _) => Exit,
        _ => DidntTakeTurn,
    }
}

fn inventory_menu(inventory: &[Object], header: &str, root: &mut Console) -> Option<usize> {
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
    fn add<T: Into<String>>(&mut self, message: T, color: [f32; 4]);
}

impl MessageLog for Vec<(String, [f32; 4])> {
    fn add<T: Into<String>>(&mut self, message: T, color: [f32; 4]) {
        if self.len() == MSG_HEIGHT {
            self.remove(0);
        }
        self.push((message.into(), color));
    }
}

fn next_level(game: &mut Game, objects: &mut Vec<Object>, ui: &mut Ui) {
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
    initialize_fov(&game.map, ui);
}

fn level_up(objects: &mut [Object], game: &mut Game, ui: &mut Ui) {
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
                &mut ui.root,
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

fn msgbox(text: &str, width: i32, root: &mut Console) {
    let options: &[&str] = &[];
    menu(text, options, width, root);
}
