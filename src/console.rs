pub type Cell = (char, [f32;4]);

pub struct Console {
    buffer: Vec<Vec<Cell>>,
}

impl Console {
    pub fn new(h: usize, w: usize) -> Console {
        Console {
            buffer: vec![vec![(' ', [0.0; 4]); h]; w],
        }
    }

    pub fn print(&mut self, x: usize, y: usize, color: [f32;4], text: &str) {
        for (c, i) in text.iterate() {
            buffer[x + i][y] = (c, color);
        }
    }
    
    pub fn print_centered(&mut self, x: usize, y: usize, color: [f32;4], text: &str) {
        self.print(x - text.len() / 2, y, color, text);
    }
}