use map::*;
use object::*;
use ::*;

const COLOR_DARK_WALL: [f32; 4] = [0.0,0.0, 0.39, 1.0];
const COLOR_LIGHT_WALL: [f32; 4] = [0.51, 0.43, 0.2, 1.0];
const COLOR_DARK_GROUND: [f32; 4] = [0.2, 0.2, 0.59, 1.0];
const COLOR_LIGHT_GROUND: [f32; 4] = [0.78, 0.7, 0.2, 1.0];

const TORCH_RADIUS: i32 = 10;

pub const PANEL_HEIGHT: i32 = 7;
const PANEL_Y: i32 = SCREEN_HEIGHT - PANEL_HEIGHT;
const BAR_WIDTH: i32 = 20;

pub const MSG_HEIGHT: usize = PANEL_HEIGHT as usize - 1;
const MSG_WIDTH: i32 = SCREEN_WIDTH - BAR_WIDTH - 2;
const MSG_X: i32 = BAR_WIDTH + 2;

pub const INVENTORY_WIDTH: i32 = 50;

pub fn render_all(ui: &mut Ui, objects: &[Object], game: &mut Game, fov_recompute: bool) {
    // TODO: Make render not take mutable references.
    if fov_recompute {
        let player = &objects[PLAYER];
        ui.fov.recompute(player.x, player.y, TORCH_RADIUS);

        for y in 0..MAP_HEIGHT {
            for x in 0..MAP_WIDTH {
                let visible = ui.fov.is_in_fov(x, y);
                let wall = game.map[x as usize][y as usize].block_sight;
                let color = match (visible, wall) {
                    (false, true) => COLOR_DARK_WALL,
                    (false, false) => COLOR_DARK_GROUND,
                    (true, true) => COLOR_LIGHT_WALL,
                    (true, false) => COLOR_LIGHT_GROUND,
                };

                let explored = &mut game.map[x as usize][y as usize].explored;
                if visible {
                    *explored = true;
                }
                if *explored {
                    ui.con
                        .set_char_background(x, y, color, BackgroundFlag::Set);
                }
            }
        }
    }

    let mut to_draw: Vec<_> = objects
        .iter()
        .filter(|o| {
            ui.fov.is_in_fov(o.x, o.y)
                || (o.always_visible && game.map[o.x as usize][o.y as usize].explored)
        })
        .collect();
    to_draw.sort_by(|o1, o2| o1.blocks.cmp(&o2.blocks));
    for object in &to_draw {
        object.draw(&mut ui.con);
    }

    blit(
        &ui.con,
        (0, 0),
        (MAP_WIDTH, MAP_HEIGHT),
        &mut ui.root,
        (0, 0),
        1.0,
        1.0,
    );

    ui.panel.set_default_background(colors::BLACK);
    ui.panel.clear();

    // print the game messages, one line at a time
    let mut y = MSG_HEIGHT as i32;
    for &(ref msg, color) in game.log.iter().rev() {
        let msg_height = ui.panel.get_height_rect(MSG_X, y, MSG_WIDTH, 0, msg);
        y -= msg_height;
        if y < 0 {
            break;
        }
        ui.panel.set_default_foreground(color);
        ui.panel.print_rect(MSG_X, y, MSG_WIDTH, 0, msg);
    }

    // show the player's stats
    let hp = objects[PLAYER].fighter.map_or(0, |f| f.hp);
    let max_hp = objects[PLAYER].max_hp(game);
    render_bar(
        &mut ui.panel,
        1,
        1,
        BAR_WIDTH,
        "HP",
        hp,
        max_hp,
        colors::LIGHT_RED,
        colors::DARKER_RED,
    );

    ui.panel.print_ex(
        1,
        3,
        BackgroundFlag::None,
        TextAlignment::Left,
        format!("Dungeon level: {}", game.dungeon_level),
    );

    // Display names of objects under the mouse
    ui.panel.set_default_foreground(colors::LIGHT_GREY);
    ui.panel.print_ex(
        1,
        0,
        BackgroundFlag::None,
        TextAlignment::Left,
        get_names_under_mouse(ui.mouse, objects, &ui.fov),
    );

    blit(
        &ui.panel,
        (0, 0),
        (SCREEN_WIDTH, PANEL_HEIGHT),
        &mut ui.root,
        (0, PANEL_Y),
        1.0,
        1.0,
    );
}

fn render_bar(
    panel: &mut Console,
    x: i32,
    y: i32,
    total_width: i32,
    name: &str,
    value: i32,
    maximum: i32,
    bar_color: [f32; 4],
    back_color: [f32; 4],
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

fn get_names_under_mouse(mouse: Mouse, objects: &[Object], fov: &Fov) -> String {
    let (x, y) = (mouse.cx as i32, mouse.cy as i32);
    objects
        .iter()
        .filter(|obj| obj.pos() == (x, y) && fov.is_in_fov(obj.x, obj.y))
        .map(|obj| obj.name.clone())
        .collect::<Vec<_>>()
        .join(", ")
}

pub fn menu<T: AsRef<str>>(
    header: &str,
    options: &[T],
    width: i32,
    root: &mut Console,
) -> Option<usize> {
    assert!(
        options.len() <= 26,
        "Cannot have a menu with more than 26 options."
    );

    // Calculate total height for header and contents
    let header_height = if header.is_empty() {
        0
    } else {
        root.get_height_rect(0, 0, width, SCREEN_HEIGHT, header)
    };
    let height = options.len() as i32 + header_height;

    let mut window = Console::new(width, height);
    window.set_default_foreground(colors::WHITE);
    window.print_rect_ex(
        0,
        0,
        width,
        height,
        BackgroundFlag::None,
        TextAlignment::Left,
        header,
    );

    for (index, option_text) in options.iter().enumerate() {
        let menu_letter = (b'a' + index as u8) as char;
        let text = format!("({}) {}", menu_letter, option_text.as_ref());
        window.print_ex(
            0,
            header_height + index as i32,
            BackgroundFlag::None,
            TextAlignment::Left,
            text,
        );
    }

    let x = SCREEN_WIDTH / 2 - width / 2;
    let y = SCREEN_HEIGHT / 2 - height / 2;
    blit(
        &window,
        (0, 0),
        (MAP_WIDTH, MAP_HEIGHT),
        root,
        (x, y),
        1.0,
        0.7,
    );

    root.flush();
    let key = root.wait_for_keypress(true);
    if key.printable.is_alphabetic() {
        let index = key.printable.to_ascii_lowercase() as usize - 'a' as usize;
        if index < options.len() {
            Some(index)
        } else {
            None
        }
    } else {
        None
    }
}
