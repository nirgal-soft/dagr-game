use anyhow::Result;
use dagr_lib::world::HexProfile;
use rand::{Rng, SeedableRng, rngs::StdRng};

use crate::areas::{Area, Feature, Ground, Pos};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TerrainProfile {
    Mountains,
    Hills,
    Plains,
    Swamp,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VegetationProfile {
    DenseForest,
    LightForest,
    Grassland,
    Barren,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WaterProfile {
    Lake,
    River,
    Dry,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WildernessProfile {
    pub terrain: TerrainProfile,
    pub vegetation: VegetationProfile,
    pub water: WaterProfile,
}

impl WildernessProfile {
    pub fn from_hex(profile: &HexProfile) -> Self {
        Self {
            terrain: match short_name(profile.terrain.as_str()) {
                "mountains" => TerrainProfile::Mountains,
                "plains" => TerrainProfile::Plains,
                "swamp" => TerrainProfile::Swamp,
                _ => TerrainProfile::Hills,
            },
            vegetation: match short_name(profile.vegetation.as_str()) {
                "dense_forest" => VegetationProfile::DenseForest,
                "grassland" => VegetationProfile::Grassland,
                "barren" => VegetationProfile::Barren,
                _ => VegetationProfile::LightForest,
            },
            water: match short_name(profile.water.as_str()) {
                "lake" => WaterProfile::Lake,
                "river" => WaterProfile::River,
                _ => WaterProfile::Dry,
            },
        }
    }

    fn ground(self) -> Ground {
        match self.terrain {
            TerrainProfile::Mountains => Ground::MOUNTAIN,
            TerrainProfile::Hills => Ground::HILLS,
            TerrainProfile::Plains => Ground::PLAINS,
            TerrainProfile::Swamp => Ground::SWAMP,
        }
    }
}

impl Default for WildernessProfile {
    fn default() -> Self {
        Self {
            terrain: TerrainProfile::Hills,
            vegetation: VegetationProfile::LightForest,
            water: WaterProfile::Dry,
        }
    }
}

fn short_name(key: &str) -> &str {
    key.rsplit(':').next().unwrap_or(key)
}

pub struct WildernessGenerator {
    pub seed: u64,
    pub profile: WildernessProfile,
}

impl WildernessGenerator {
    pub fn new(seed: u64) -> Self {
        Self {
            seed,
            profile: WildernessProfile::default(),
        }
    }

    pub fn for_hex(seed: u64, profile: &HexProfile) -> Self {
        Self {
            seed,
            profile: WildernessProfile::from_hex(profile),
        }
    }

    pub fn generate(&self, width: i32, height: i32) -> Result<Area> {
        let mut rng = StdRng::seed_from_u64(self.seed);
        let mut area = Area::wilderness(width, height);
        area.set_ground(self.profile.ground());

        self.paint_vegetation(&mut area, &mut rng);
        self.paint_terrain(&mut area, &mut rng);
        self.paint_water(&mut area, &mut rng);

        let center = (width / 2, height / 2);
        clear_radius(&mut area, center, 1);
        area.set_entrance(center.0, center.1);
        self.place_landmark(&mut area, &mut rng);
        Ok(area)
    }

    fn paint_vegetation(&self, area: &mut Area, rng: &mut StdRng) {
        let scale = ((area.width * area.height) / 1200).max(1);
        let (blobs, steps, brush): (i32, i32, i32) = match self.profile.vegetation {
            VegetationProfile::DenseForest => (scale * 4, 28, 2),
            VegetationProfile::LightForest => (scale * 3, 20, 2),
            VegetationProfile::Grassland => (scale, 10, 1),
            VegetationProfile::Barren => (0, 0, 0),
        };
        paint_blobs(
            area,
            rng,
            Feature::BRUSH,
            blobs,
            steps + 8,
            brush.saturating_add(1),
        );
        paint_blobs(area, rng, Feature::TREE, blobs, steps, brush);
    }

    fn paint_terrain(&self, area: &mut Area, rng: &mut StdRng) {
        let scale = ((area.width * area.height) / 1400).max(1);
        let (blobs, steps, brush): (i32, i32, i32) = match self.profile.terrain {
            TerrainProfile::Mountains => (scale * 4, 24, 2),
            TerrainProfile::Hills => (scale * 2, 16, 2),
            TerrainProfile::Plains => (scale, 7, 1),
            TerrainProfile::Swamp => (scale, 8, 1),
        };
        paint_blobs(
            area,
            rng,
            Feature::RUBBLE,
            blobs,
            steps + 5,
            brush.saturating_add(1),
        );
        paint_blobs(area, rng, Feature::ROCK, blobs, steps, brush);
    }

    fn paint_water(&self, area: &mut Area, rng: &mut StdRng) {
        match self.profile.water {
            WaterProfile::Lake => {
                let scale = ((area.width * area.height) / 1800).max(1);
                paint_blobs(area, rng, Feature::WATER, scale * 2, 42, 3);
            }
            WaterProfile::River => paint_river(area, rng),
            WaterProfile::Dry if self.profile.terrain == TerrainProfile::Swamp => {
                let scale = ((area.width * area.height) / 1600).max(1);
                paint_blobs(area, rng, Feature::WATER, scale * 2, 20, 2);
            }
            WaterProfile::Dry => {}
        }
    }

    fn place_landmark(&self, area: &mut Area, rng: &mut StdRng) {
        if area.width < 7 || area.height < 7 {
            return;
        }
        let center = (area.width / 2, area.height / 2);
        for _ in 0..40 {
            let landmark = (
                rng.random_range(2..area.width - 2),
                rng.random_range(2..area.height - 2),
            );
            let distance = (landmark.0 - center.0).abs() + (landmark.1 - center.1).abs();
            if distance > 8 && distance <= 16 && area.is_walkable(landmark.0, landmark.1) {
                area.set_feature(landmark.0, landmark.1, Feature::LANDMARK);
                return;
            }
        }
        let fallback = (2, 2);
        area.set_feature(fallback.0, fallback.1, Feature::LANDMARK);
    }
}

fn paint_blobs(
    area: &mut Area,
    rng: &mut StdRng,
    feature: Feature,
    count: i32,
    steps: i32,
    brush: i32,
) {
    if area.width <= 4 || area.height <= 4 {
        return;
    }
    for _ in 0..count {
        let mut cursor = (
            rng.random_range(2..area.width - 2),
            rng.random_range(2..area.height - 2),
        );
        for _ in 0..steps {
            for dy in -brush..=brush {
                for dx in -brush..=brush {
                    if dx * dx + dy * dy <= brush * brush && rng.random_bool(0.72) {
                        let x = cursor.0 + dx;
                        let y = cursor.1 + dy;
                        if x > 0 && x < area.width - 1 && y > 0 && y < area.height - 1 {
                            area.set_feature(x, y, feature);
                        }
                    }
                }
            }
            cursor.0 = (cursor.0 + rng.random_range(-1..=1)).clamp(2, area.width - 3);
            cursor.1 = (cursor.1 + rng.random_range(-1..=1)).clamp(2, area.height - 3);
        }
    }
}

fn paint_river(area: &mut Area, rng: &mut StdRng) {
    if area.width < 5 || area.height < 5 {
        return;
    }
    if rng.random_bool(0.5) {
        let mut y = rng.random_range(2..area.height - 2);
        for x in 1..area.width - 1 {
            area.set_feature(x, y, Feature::WATER);
            if rng.random_bool(0.45) {
                y = (y + rng.random_range(-1..=1)).clamp(1, area.height - 2);
            }
        }
    } else {
        let mut x = rng.random_range(2..area.width - 2);
        for y in 1..area.height - 1 {
            area.set_feature(x, y, Feature::WATER);
            if rng.random_bool(0.45) {
                x = (x + rng.random_range(-1..=1)).clamp(1, area.width - 2);
            }
        }
    }
}

fn clear_radius(area: &mut Area, center: Pos, radius: i32) {
    for dy in -radius..=radius {
        for dx in -radius..=radius {
            area.remove_feature(center.0 + dx, center.1 + dy);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn count(area: &Area, feature: Feature) -> usize {
        (0..area.height)
            .flat_map(|y| (0..area.width).map(move |x| (x, y)))
            .filter(|(x, y)| area.get_feature(*x, *y) == Some(&feature))
            .count()
    }

    #[test]
    fn generation_is_deterministic_with_natural_open_edges() {
        let generator = WildernessGenerator::new(1234);
        let first = generator.generate(64, 40).unwrap();
        let second = generator.generate(64, 40).unwrap();

        for y in 0..40 {
            for x in 0..64 {
                assert_eq!(first.get_tile(x, y), second.get_tile(x, y));
            }
        }
        assert_eq!(first.entrance, Some((32, 20)));
        assert_eq!(count(&first, Feature::LANDMARK), 1);
        assert_eq!(first.get_feature(1, 0), None);
        assert_eq!(first.get_feature(first.width - 2, first.height - 1), None);
    }

    #[test]
    fn parent_profile_materially_changes_generation() {
        let dense = WildernessGenerator {
            seed: 77,
            profile: WildernessProfile {
                terrain: TerrainProfile::Hills,
                vegetation: VegetationProfile::DenseForest,
                water: WaterProfile::Dry,
            },
        }
        .generate(64, 40)
        .unwrap();
        let plains = WildernessGenerator {
            seed: 77,
            profile: WildernessProfile {
                terrain: TerrainProfile::Plains,
                vegetation: VegetationProfile::Grassland,
                water: WaterProfile::Dry,
            },
        }
        .generate(64, 40)
        .unwrap();
        assert!(count(&dense, Feature::TREE) > count(&plains, Feature::TREE));
        assert_ne!(dense.get_tile(10, 10), plains.get_tile(10, 10));
    }

    #[test]
    fn river_and_lake_profiles_create_coherent_water() {
        for water in [WaterProfile::River, WaterProfile::Lake] {
            let area = WildernessGenerator {
                seed: 99,
                profile: WildernessProfile {
                    terrain: TerrainProfile::Plains,
                    vegetation: VegetationProfile::Grassland,
                    water,
                },
            }
            .generate(64, 40)
            .unwrap();
            assert!(count(&area, Feature::WATER) > 20);
        }
    }
}
