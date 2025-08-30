#[derive(Debug, Default, Clone)]
pub struct PotManager {
    main: u32,
    sides: Vec<u32>,
}

impl PotManager {
    pub fn from_contributions(contrib: [u32; 2]) -> Self {
        let a = contrib[0];
        let b = contrib[1];
        let shared = a.min(b);
        let main = shared * 2;
        let mut sides = Vec::new();
        if a != b {
            let side = a.max(b) - shared;
            if side > 0 { sides.push(side); }
        }
        Self { main, sides }
    }

    pub fn main_pot(&self) -> u32 { self.main }
    pub fn side_pots(&self) -> &[u32] { &self.sides }
}

