#[derive(Clone,Copy)]
pub struct Position {
    x: f64,
    y: f64,
}
impl Position {
    #[inline]
    pub const fn new(x: f64, y: f64) -> Position {
        Position { x, y }
    }

    #[inline] pub const fn x(&self) -> f64 { self.x }
    #[inline] pub const fn y(&self) -> f64 { self.y }

}

#[derive(Copy,Clone)]
pub struct Size {
    width: f64,
    height: f64,
}

impl Size {
    #[inline] pub const fn new(width: f64, height: f64) -> Size {
        Size { width, height }
    }

    #[inline] pub const fn width(&self) -> f64 { self.width }
    #[inline] pub const fn height(&self) -> f64 { self.height }
}