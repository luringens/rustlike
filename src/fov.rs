use map::{Map, MAP_HEIGHT, MAP_WIDTH};

#[derive(Clone, Debug)]
struct FovTile {
    pub visible: bool,
    pub blocks: bool,
}

/// Calculates and provides a FOV map.
/// Algorithm taken from http://www.roguebasin.com/index.php?title=Eligloscode
#[derive(Debug)]
pub struct Fov {
    fovmap: Vec<Vec<FovTile>>,
}

impl Fov {
    /// Creates a new blank FOV map using the default size from the Map module.
    pub fn new() -> Fov {
        Fov {
            fovmap: vec![
                vec![
                    FovTile {
                        visible: false,
                        blocks: false,
                    };
                    MAP_HEIGHT as usize
                ];
                MAP_WIDTH as usize
            ],
        }
    }

    /// Creates a FOV map, using a Map to determine what tiles block vision.
    pub fn from_map(map: &Map) -> Fov {
        let mut fovmap = vec![
            vec![
                FovTile {
                    visible: false,
                    blocks: false,
                };
                MAP_HEIGHT as usize
            ];
            MAP_WIDTH as usize
        ];
        for y in 0..MAP_HEIGHT as usize {
            for x in 0..MAP_WIDTH as usize {
                fovmap[x][y].blocks = map[x][y].block_sight;
            }
        }
        Fov { fovmap: fovmap }
    }

    /// Recomputes FOV from an origin, out to the distance of the radius.
    pub fn recompute(&mut self, origin_x: i32, origin_y: i32, radius: i32) {
        self.reset();

        for i in 0..360 {
            let dir_x = (i as f32 * 0.01745).cos();
            let dir_y = (i as f32 * 0.01745).sin();
            self.cast_fov(origin_x, origin_y, dir_x, dir_y, radius);
        }
    }

    /// Casts fov from a coordinate towards another for the length of the radius.
    fn cast_fov(&mut self, origin_x: i32, origin_y: i32, dir_x: f32, dir_y: f32, radius: i32) {
        let mut ox = origin_x as f32 + 0.5;
        let mut oy = origin_y as f32 + 0.5;
        for _ in 0..radius {
            self.fovmap[ox as usize][oy as usize].visible = true;
            if self.fovmap[ox as usize][oy as usize].blocks {
                return;
            }
            ox += dir_x;
            oy += dir_y;
        }
    }

    /// Resets all tiles to hidden.
    pub fn reset(&mut self) {
        for a in &mut self.fovmap {
            for b in a {
                b.visible = false;
            }
        }
    }

    /// Returns whether or not the coordinate is in FOV.
    pub fn is_in_fov(&self, x: i32, y: i32) -> bool {
        self.fovmap[x as usize][y as usize].visible
    }
}
