use super::contant::UNINIT_POINT_32;

#[derive(Debug)]
pub struct Position {
    x: u32,
    y: u32,
    z: u32,
    w: u32,
}

impl Position {
    fn new() -> Self {
        Position {
            x: UNINIT_POINT_32,
            y: UNINIT_POINT_32,
            z: UNINIT_POINT_32,
            w: UNINIT_POINT_32,
        }
    }

    fn operator_eq(&self, other: &Position) -> bool {
        self.x == other.x && self.y == other.y && self.z == other.z
    }

    fn operator_neq(&self, other: &Position) -> bool {
        self.x != other.x || self.y != other.y || self.z != other.z
    }

    fn is_valid(&self) -> bool {
        self.x != UNINIT_POINT_32 && self.y != UNINIT_POINT_32 && self.z != UNINIT_POINT_32
    }

    fn to_string(&self) -> String {
        format!("[ {} , {} , {} ]", self.x, self.y, self.z)
    }
}
