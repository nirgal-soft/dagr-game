use std::collections::HashMap;

use dagr_lib::world::HexView;

#[derive(Clone, Debug)]
pub struct WorldMap {
    grid: HashMap<(i32, i32), HexView>,
}

impl WorldMap {
    pub fn new() -> Self {
        Self {
            grid: HashMap::new(),
        }
    }

    pub fn insert(&mut self, hex: HexView) {
        self.grid.insert((hex.coordinate.x, hex.coordinate.y), hex);
    }

    pub fn get(&self, pos: (i32, i32)) -> Option<&HexView> {
        self.grid.get(&pos)
    }

    pub fn clear(&mut self) {
        self.grid.clear();
    }

    pub fn count(&self) -> usize {
        self.grid.len()
    }
}
