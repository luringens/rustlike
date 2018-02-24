use map::*;
use object::*;

use tcod::console::*;
use tcod::colors::{self, Color};
use tcod::map::{FovAlgorithm, Map as FovMap};
use tcod::input::Mouse;

use {Messages, PLAYER, SCREEN_HEIGHT, SCREEN_WIDTH};

const COLOR_DARK_WALL: Color = Color { r: 0, g: 0, b: 100 };
const COLOR_LIGHT_WALL: Color = Color {
    r: 130,
    g: 110,
    b: 50,
};
const COLOR_DARK_GROUND: Color = Color {
    r: 50,
    g: 50,
    b: 150,
};
const COLOR_LIGHT_GROUND: Color = Color {
    r: 200,
    g: 180,
    b: 50,
};

const FOV_ALGO: FovAlgorithm = FovAlgorithm::Basic;
const FOV_LIGHT_WALLS: bool = true;
const TORCH_RADIUS: i32 = 10;

pub const PANEL_HEIGHT: i32 = 7;
const PANEL_Y: i32 = SCREEN_HEIGHT - PANEL_HEIGHT;
const BAR_WIDTH: i32 = 20;

pub const MSG_HEIGHT: usize = PANEL_HEIGHT as usize - 1;
const MSG_WIDTH: i32 = SCREEN_WIDTH - BAR_WIDTH - 2;
const MSG_X: i32 = BAR_WIDTH + 2;

pub fn render_all(
    root: &mut Root,
    con: &mut Offscreen,
    panel: &mut Offscreen,
    objects: &[Object],
    map: &mut Map,
    messages: &Messages,
    fov_map: &mut FovMap,
    fov_recompute: bool,
    mouse: Mouse,
) {
    // TODO: Make render not take mutable references.
    if fov_recompute {
        let player = &objects[PLAYER];
        fov_map.compute_fov(player.x, player.y, TORCH_RADIUS, FOV_LIGHT_WALLS, FOV_ALGO);

        for y in 0..MAP_HEIGHT {
            for x in 0..MAP_WIDTH {
                let visible = fov_map.is_in_fov(x, y);
                let wall = map[x as usize][y as usize].block_sight;
                let color = match (visible, wall) {
                    (false, true) => COLOR_DARK_WALL,
                    (false, false) => COLOR_DARK_GROUND,
                    (true, true) => COLOR_LIGHT_WALL,
                    (true, false) => COLOR_LIGHT_GROUND,
                };

                let explored = &mut map[x as usize][y as usize].explored;
                if visible {
                    *explored = true;
                }
                if *explored {
                    con.set_char_background(x, y, color, BackgroundFlag::Set);
                }
            }
        }
    }

    let mut to_draw: Vec<_> = objects
        .iter()
        .filter(|o| fov_map.is_in_fov(o.x, o.y))
        .collect();
    to_draw.sort_by(|o1, o2| o1.blocks.cmp(&o2.blocks));
    for object in &to_draw {
        if fov_map.is_in_fov(object.x, object.y) {
            object.draw(con);
        }
    }

    blit(con, (0, 0), (MAP_WIDTH, MAP_HEIGHT), root, (0, 0), 1.0, 1.0);

    panel.set_default_background(colors::BLACK);
    panel.clear();

    // print the game messages, one line at a time
    let mut y = MSG_HEIGHT as i32;
    for &(ref msg, color) in messages.iter().rev() {
        let msg_height = panel.get_height_rect(MSG_X, y, MSG_WIDTH, 0, msg);
        y -= msg_height;
        if y < 0 {
            break;
        }
        panel.set_default_foreground(color);
        panel.print_rect(MSG_X, y, MSG_WIDTH, 0, msg);
    }

    // show the player's stats
    let hp = objects[PLAYER].fighter.map_or(0, |f| f.hp);
    let max_hp = objects[PLAYER].fighter.map_or(0, |f| f.max_hp);
    render_bar(
        panel,
        1,
        1,
        BAR_WIDTH,
        "HP",
        hp,
        max_hp,
        colors::LIGHT_RED,
        colors::DARKER_RED,
    );

    // Display names of objects under the mouse
    panel.set_default_foreground(colors::LIGHT_GREY);
    panel.print_ex(
        1,
        0,
        BackgroundFlag::None,
        TextAlignment::Left,
        get_names_under_mouse(mouse, objects, fov_map),
    );

    blit(
        panel,
        (0, 0),
        (SCREEN_WIDTH, PANEL_HEIGHT),
        root,
        (0, PANEL_Y),
        1.0,
        1.0,
    );
}

fn render_bar(
    panel: &mut Offscreen,
    x: i32,
    y: i32,
    total_width: i32,
    name: &str,
    value: i32,
    maximum: i32,
    bar_color: Color,
    back_color: Color,
) {
    // render a bar (HP, experience, etc). First calculate the width of the bar
    let bar_width = (value as f32 / maximum as f32 * total_width as f32) as i32;

    // render the background first
    panel.set_default_background(back_color);
    panel.rect(x, y, total_width, 1, false, BackgroundFlag::Screen);

    // now render the bar on top
    panel.set_default_background(bar_color);
    if bar_width > 0 {
        panel.rect(x, y, bar_width, 1, false, BackgroundFlag::Screen);
    }

    // finally, some centered text with the values
    panel.set_default_foreground(colors::WHITE);
    panel.print_ex(
        x + total_width / 2,
        y,
        BackgroundFlag::None,
        TextAlignment::Center,
        &format!("{}: {}/{}", name, value, maximum),
    );
}

fn get_names_under_mouse(mouse: Mouse, objects: &[Object], fov_map: &FovMap) -> String {
    let (x, y) = (mouse.cx as i32, mouse.cy as i32);
    objects
        .iter()
        .filter(|obj| obj.pos() == (x, y) && fov_map.is_in_fov(obj.x, obj.y))
        .map(|obj| obj.name.clone())
        .collect::<Vec<_>>()
        .join(", ")
}
