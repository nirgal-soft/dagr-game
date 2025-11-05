pub struct BorderStyle{
  pub top_left: char,
  pub top_right: char,
  pub bottom_left: char,
  pub bottom_right: char,
  pub horizontal: char,
  pub vertical: char,
}

impl BorderStyle{
  pub const SINGLE: Self = Self{
    top_left: '┌',
    top_right: '┐',
    bottom_left: '└',
    bottom_right: '┘',
    horizontal: '─',
    vertical: '│',
  };

  pub const DOUBLE: Self = Self{
    top_left: '╔',
    top_right: '╗',
    bottom_left: '╚',
    bottom_right: '╝',
    horizontal: '═',
    vertical: '║',
  };
}
