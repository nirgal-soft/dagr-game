use crate::areas::Pos;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InspectionEntry {
    pub name: String,
    pub description: String,
}

impl InspectionEntry {
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InspectionReport {
    pub pos: Pos,
    pub entries: Vec<InspectionEntry>,
}

impl InspectionReport {
    pub fn lines(&self) -> Vec<String> {
        let mut lines = vec![format!("tile: ({}, {})", self.pos.0, self.pos.1)];
        for entry in &self.entries {
            lines.push(entry.name.clone());
            lines.push(format!("  {}", entry.description));
        }
        lines
    }
}
