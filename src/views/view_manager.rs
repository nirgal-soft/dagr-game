use std::collections::HashMap;

use dagr_lib::world::LocationId;

use crate::areas::Area;
use crate::wilderness_layout::WildernessLayout;

use super::area_key::AreaKey;
use super::view_mode::ViewMode;

pub struct ViewManager {
    pub mode: ViewMode,
    area_cache: HashMap<AreaKey, Area>,
    wilderness_layout: WildernessLayout,
}

impl ViewManager {
    pub fn new(wilderness_layout: WildernessLayout) -> Self {
        Self {
            mode: ViewMode::World,
            area_cache: HashMap::new(),
            wilderness_layout,
        }
    }

    pub fn current_area(&self) -> Option<&Area> {
        let key: Option<AreaKey> = self.mode.into();
        key.and_then(|key| self.area_cache.get(&key))
    }

    pub fn current_area_mut(&mut self) -> Option<&mut Area> {
        let key: Option<AreaKey> = self.mode.into();
        key.and_then(|key| self.area_cache.get_mut(&key))
    }

    pub fn is_in_world(&self) -> bool {
        matches!(self.mode, ViewMode::World)
    }

    pub fn current_location(&self) -> Option<LocationId> {
        match self.mode {
            ViewMode::World => None,
            ViewMode::Location { location, .. } => Some(location),
        }
    }

    pub fn current_level(&self) -> Option<i32> {
        match self.mode {
            ViewMode::World => None,
            ViewMode::Location { level, .. } => level,
        }
    }

    pub fn show_world(&mut self) {
        self.mode = ViewMode::World;
    }

    pub fn show_area(&mut self, location: LocationId, level: Option<i32>, area: Area) {
        self.area_cache.insert(AreaKey::new(location, level), area);
        self.mode = match level {
            Some(level) => ViewMode::location_with_level(location, level),
            None => ViewMode::location(location),
        };
    }

    pub fn cached_area(&mut self, location: LocationId, level: Option<i32>) -> bool {
        let key = AreaKey::new(location, level);
        if self.area_cache.contains_key(&key) {
            self.mode = match level {
                Some(level) => ViewMode::location_with_level(location, level),
                None => ViewMode::location(location),
            };
            true
        } else {
            false
        }
    }

    pub fn wilderness_dimensions(&self) -> (i32, i32) {
        (
            self.wilderness_layout.area_width,
            self.wilderness_layout.area_height,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returning_to_an_area_preserves_exploration_state() {
        let location = LocationId::new(1).unwrap();
        let mut manager = ViewManager::new(WildernessLayout::new(1, 1, 10, 10).unwrap());
        manager.show_area(location, None, Area::wilderness(10, 10));
        manager.current_area_mut().unwrap().update_visibility(5, 5);
        manager.show_world();

        assert!(manager.cached_area(location, None));
        assert!(manager.current_area().unwrap().is_seen(5, 5));
    }
}
