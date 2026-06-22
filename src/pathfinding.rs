use std::collections::{BinaryHeap, HashMap, HashSet};
use std::cmp::Ordering;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Node {
    pub x: i32,
    pub y: i32,
}

impl Node {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    pub fn from_world(wx: f32, wy: f32) -> Self {
        Self {
            x: (wx + 0.5).floor() as i32,
            y: (wy + 0.5).floor() as i32,
        }
    }

    pub fn to_world(self) -> (f32, f32) {
        (self.x as f32, self.y as f32)
    }

    fn manhattan(self, other: Self) -> i32 {
        (self.x - other.x).abs() + (self.y - other.y).abs()
    }

    fn neighbors(self) -> [Self; 4] {
        [
            Self::new(self.x + 1, self.y),
            Self::new(self.x - 1, self.y),
            Self::new(self.x, self.y + 1),
            Self::new(self.x, self.y - 1),
        ]
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct State {
    cost: i32,
    node: Node,
}

impl Ord for State {
    fn cmp(&self, other: &Self) -> Ordering {
        other.cost.cmp(&self.cost)
    }
}

impl PartialOrd for State {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub fn find_path(
    walkable: &HashSet<Node>,
    start: Node,
    goal: Node,
) -> Option<Vec<Node>> {
    if !walkable.contains(&start) || !walkable.contains(&goal) {
        return None;
    }

    let mut open = BinaryHeap::new();
    let mut came_from: HashMap<Node, Node> = HashMap::new();
    let mut g_score: HashMap<Node, i32> = HashMap::new();
    let mut f_score: HashMap<Node, i32> = HashMap::new();

    let h = start.manhattan(goal);
    g_score.insert(start, 0);
    f_score.insert(start, h);
    open.push(State { cost: h, node: start });

    while let Some(current) = open.pop() {
        if current.node == goal {
            let mut path = Vec::new();
            let mut n = goal;
            path.push(n);
            while let Some(&prev) = came_from.get(&n) {
                path.push(prev);
                n = prev;
            }
            path.reverse();
            return Some(path);
        }

        if current.cost > f_score.get(&current.node).copied().unwrap_or(i32::MAX) {
            continue;
        }

        for neighbor in current.node.neighbors() {
            if !walkable.contains(&neighbor) {
                continue;
            }
            let tentative = g_score[&current.node] + 1;
            if tentative < g_score.get(&neighbor).copied().unwrap_or(i32::MAX) {
                came_from.insert(neighbor, current.node);
                g_score.insert(neighbor, tentative);
                let f = tentative + neighbor.manhattan(goal);
                f_score.insert(neighbor, f);
                open.push(State { cost: f, node: neighbor });
            }
        }
    }

    None
}
