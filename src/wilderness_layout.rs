use anyhow::{bail, Context, Result};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WildernessLayout {
  pub columns: i32,
  pub rows: i32,
  pub area_width: i32,
  pub area_height: i32,
}

impl WildernessLayout {
  pub fn new(columns: i32, rows: i32, area_width: i32, area_height: i32) -> Result<Self> {
    if columns <= 0 || rows <= 0 {
      bail!("wilderness area grid must have positive columns and rows")
    }
    if area_width <= 0 || area_height <= 0 {
      bail!("wilderness areas must have positive tile dimensions")
    }
    Ok(Self { columns, rows, area_width, area_height })
  }

  pub fn from_env() -> Result<Self> {
    Self::new(
      env_i32("WILDERNESS_AREA_COLUMNS", Self::default().columns)?,
      env_i32("WILDERNESS_AREA_ROWS", Self::default().rows)?,
      env_i32("WILDERNESS_AREA_WIDTH", Self::default().area_width)?,
      env_i32("WILDERNESS_AREA_HEIGHT", Self::default().area_height)?,
    )
  }

  pub fn min_x(self) -> i32 {-(self.columns / 2)}
  pub fn min_y(self) -> i32 {-(self.rows / 2)}
  pub fn max_x(self) -> i32 {self.min_x() + self.columns - 1}
  pub fn max_y(self) -> i32 {self.min_y() + self.rows - 1}

  pub fn contains(self, x: i32, y: i32) -> bool {
    (self.min_x()..=self.max_x()).contains(&x)
      && (self.min_y()..=self.max_y()).contains(&y)
  }

  pub fn bounds_label(self) -> String {
    format!("x={}..{}, y={}..{}", self.min_x(), self.max_x(), self.min_y(), self.max_y())
  }
}

impl Default for WildernessLayout {
  fn default() -> Self {
    Self { columns: 3, rows: 3, area_width: 64, area_height: 40 }
  }
}

fn env_i32(name: &str, default: i32) -> Result<i32> {
  match std::env::var(name) {
    Ok(value) => value.parse::<i32>().with_context(|| format!("{name} must be an integer")),
    Err(std::env::VarError::NotPresent) => Ok(default),
    Err(error) => Err(error).with_context(|| format!("failed to read {name}")),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn centered_layout_has_finite_bounds() {
    let layout = WildernessLayout::new(3, 3, 64, 40).unwrap();
    assert_eq!((layout.min_x(), layout.max_x()), (-1, 1));
    assert_eq!((layout.min_y(), layout.max_y()), (-1, 1));
    assert!(layout.contains(1, -1));
    assert!(!layout.contains(2, 0));
  }

  #[test]
  fn even_layouts_still_include_origin() {
    let layout = WildernessLayout::new(4, 2, 20, 12).unwrap();
    assert!(layout.contains(0, 0));
    assert_eq!((layout.min_x(), layout.max_x()), (-2, 1));
  }

  #[test]
  fn invalid_layout_is_rejected() {
    assert!(WildernessLayout::new(0, 3, 64, 40).is_err());
    assert!(WildernessLayout::new(3, 3, -1, 40).is_err());
  }
}
