use std::collections::VecDeque;

use dagr_lib::combat::{AttackResult, CombatExchange};

const MAX_LINES: usize = 12;

#[derive(Default)]
pub struct CombatLog{
  lines: VecDeque<String>,
}

impl CombatLog{
  pub fn record_exchange(&mut self, enemy:&str, report:&CombatExchange){
    self.push(format_attack("You",&report.attack));
    match &report.retaliation{
      Some(result) => self.push(format_attack(enemy,result)),
      None => self.push(format!("{enemy} is defeated.")),
    }
    if report.attacker_defeated{
      self.push("You are down. Press R to reset the arena.");
    }else{
      self.push(format!("Your HP: {}",report.attacker_hp));
    }
  }

  pub fn push(&mut self,line:impl Into<String>){
    self.lines.push_back(line.into());
    while self.lines.len()>MAX_LINES{self.lines.pop_front();}
  }

  pub fn recent(&self,count:usize)->Vec<String>{
    self.lines.iter().rev().take(count).cloned().collect::<Vec<_>>()
      .into_iter().rev().collect()
  }

  pub fn clear(&mut self){self.lines.clear()}
}

fn format_attack(actor:&str,result:&AttackResult)->String{
  if result.hit{
    format!(
      "{actor}: d20 {} -> HIT {} | AC {} < roll <= AV {}",
      result.roll,result.damage,result.armor_class,result.attack_value,
    )
  }else{
    format!(
      "{actor}: d20 {} -> miss | needs AC {} < roll <= AV {}",
      result.roll,result.armor_class,result.attack_value,
    )
  }
}

#[cfg(test)]
mod tests{
  use super::*;

  #[test]
  fn log_keeps_only_recent_lines(){
    let mut log=CombatLog::default();
    for index in 0..20{log.push(index.to_string())}
    assert_eq!(log.recent(20).len(),MAX_LINES);
    assert_eq!(log.recent(1),vec!["19"]);
  }
}
