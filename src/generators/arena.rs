use crate::areas::{Area, Feature, Ground};

pub const COMBAT_ARENA_KEY: &str = "debug:combat-arena";
pub const COMBAT_ARENA_WIDTH: i32 = 31;
pub const COMBAT_ARENA_HEIGHT: i32 = 21;

pub struct ArenaGenerator;

impl ArenaGenerator {
    pub fn generate(width: i32, height: i32) -> Area {
        let mut area = Area::new(width, height, Ground::DUNGEON);
        let center = (width / 2, height / 2);
        let radius_x = (width / 2 - 2).max(1) as i64;
        let radius_y = (height / 2 - 2).max(1) as i64;
        let radius_product = radius_x * radius_x * radius_y * radius_y;

        for y in 0..height {
            for x in 0..width {
                let dx = (x - center.0) as i64;
                let dy = (y - center.1) as i64;
                let ellipse = dx * dx * radius_y * radius_y + dy * dy * radius_x * radius_x;
                if ellipse >= radius_product {
                    area.set_feature(x, y, Feature::WALL);
                }
            }
        }
        area.set_entrance(center.0, center.1);
        area
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arena_is_an_open_stone_circle() {
        let area = ArenaGenerator::generate(COMBAT_ARENA_WIDTH, COMBAT_ARENA_HEIGHT);
        let center = area.entrance.expect("arena center");
        assert!(area.is_walkable(center.0, center.1));
        assert!(!area.is_walkable(0, 0));
        assert!(!area.is_walkable(COMBAT_ARENA_WIDTH - 1, COMBAT_ARENA_HEIGHT - 1));
        assert!(
            !area.has_fov(),
            "the full debug arena should remain visible"
        );
    }
}
