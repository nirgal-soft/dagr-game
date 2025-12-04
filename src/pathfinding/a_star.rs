use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
use super::Pos;

#[derive(Clone, Eq, PartialEq)]
pub struct Node{
  pos: Pos,
  cost: i32,
  priority: i32,
}

impl Ord for Node{
  fn cmp(&self, other: &Self) -> Ordering{
    other.priority.cmp(&self.priority)
  }
}

impl PartialOrd for Node{
  fn partial_cmp(&self, other: &Self) -> Option<Ordering>{
    Some(self.cmp(other))
  }
}

fn heuristic(a: Pos, b: Pos) -> i32{
  (a.0 - b.0).abs().max((a.1 - b.1).abs())
}

pub fn find_path<F>(start: Pos, goal: Pos, is_walkable: F) -> Option<Vec<Pos>>
where 
  F : Fn(i32, i32) -> bool,
{
  if start == goal {
    return Some(vec![]);
  }

  let mut open = BinaryHeap::new();
  let mut came_from: HashMap<Pos, Pos> = HashMap::new();
  let mut cost_so_far: HashMap<Pos, i32> = HashMap::new();

  open.push(Node{
    pos: start,
    cost: 0,
    priority: heuristic(start, goal),
  });
  cost_so_far.insert(start, 0);

  let directions = [
    (-1, -1), (0, -1), (1, -1),
    (-1, 0), (1, 0),
    (-1, 1), (0, 1), (1, 1),
  ];

  while let Some(current) = open.pop(){
    if current.pos == goal{
      let mut path = vec![goal];
      let mut pos = goal;
      while let Some(&prev) = came_from.get(&pos){
        if prev == start{
          break;
        }
        path.push(prev);
        pos = prev;
      }
      path.reverse();
      return Some(path);
    }

    let current_cost = cost_so_far[&current.pos];

    for(dx, dy) in directions{
      let next = (current.pos.0 + dx, current.pos.1 + dy);

      if !is_walkable(next.0, next.1){
        continue;
      }

      let move_cost = if dx != 0 && dy != 0{14} else {10};
      let new_cost = current_cost + move_cost;

      if !cost_so_far.contains_key(&next) || new_cost < cost_so_far[&next]{
        cost_so_far.insert(next, new_cost);
        let priority = new_cost + heuristic(next, goal) * 10;
        open.push(Node{
          pos: next,
          cost: new_cost,
          priority,
        });
        came_from.insert(next, current.pos);
      }
    }
  }
  None
}

pub fn find_path_to_nearest<F, I>(
  start: Pos,
  goals: I,
  is_walkable: F,
) -> Option<(Vec<Pos> , Pos)>
where 
  F: Fn(i32, i32) -> bool,
  I: IntoIterator<Item = Pos>,
{
  let goals: Vec<_> = goals.into_iter().collect();

  if goals.is_empty(){
    return None;
  }

  let mut best_path: Option<Vec<Pos>> = None;
  let mut best_goal: Option<Pos> = None;

  for goal in goals{
    if let Some(path) = find_path(start, goal, &is_walkable){
      let dominated = best_path.as_ref().map(|bp| path.len() < bp.len()).unwrap_or(true);
      if dominated{
        best_path = Some(path);
        best_goal = Some(goal);
      }
    }
  }

  best_path.map(|p| (p, best_goal.unwrap()))
}
