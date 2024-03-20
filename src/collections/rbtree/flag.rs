use core::fmt::Display;

#[derive(Copy, Clone)]
pub struct Flag {
    pub flag: u8,
}
#[derive(Clone, Copy, PartialEq)]
pub enum Rela {
    LEFT,
    RIGHT,
    ROOT,
}
pub const LEFT: u8 = 0b00;
pub const RIGHT: u8 = 0b01;
pub const ROOT: u8 = 0b10;
impl From<u8> for Rela {
    fn from(flag: u8) -> Self {
        match flag {
            LEFT => Rela::LEFT,
            RIGHT => Rela::RIGHT,
            ROOT => Rela::ROOT,
            _ => unreachable!(),
        }
    }
}
impl Into<u8> for Rela {
    fn into(self) -> u8 {
        self as u8
    }
}
impl Into<usize> for Rela {
    fn into(self) -> usize {
        self as usize
    }
}
impl Display for Rela {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Rela::LEFT => write!(f, "L"),
            Rela::RIGHT => write!(f, "R"),
            Rela::ROOT => write!(f, "T"),
        }
    }
}
#[derive(Clone, Copy, PartialEq)]
pub enum Color {
    RED,
    BLACK,
}
pub const RED: u8 = 0b0000;
pub const BLACK: u8 = 0b100;
impl Color {
    pub fn as_u8(&self) -> u8 {
        match self {
            Color::RED => RED,
            Color::BLACK => BLACK,
        }
    }
}
impl From<u8> for Color {
    fn from(flag: u8) -> Self {
        match flag {
            RED => Color::RED,
            BLACK => Color::BLACK,
            _ => unreachable!(),
        }
    }
}
impl Into<u8> for Color {
    fn into(self) -> u8 {
        (self as u8) << 2
    }
}
impl Display for Color {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Color::RED => write!(f, "R"),
            Color::BLACK => write!(f, "B"),
        }
    }
}
impl Flag {
    #[inline(always)]
    pub fn is_red(&self) -> bool {
        self.flag & 0b0100 == RED
    }
    #[inline(always)]
    pub fn is_black(&self) -> bool {
        self.flag & 0b0100 == BLACK
    }
    #[inline(always)]
    pub fn is_root(&self) -> bool {
        self.flag & 0b10 == ROOT
    }
    #[inline(always)]
    pub fn is_left(&self) -> bool {
        self.flag & 0b11 == LEFT
    }
    #[inline(always)]
    pub fn is_right(&self) -> bool {
        self.flag & 0b11 == RIGHT
    }
    #[inline(always)]
    pub fn set_rela(&mut self, rela: u8) -> &mut Self {
        self.flag = (self.flag & 0b11111100) | rela;
        self
    }
    #[inline(always)]
    pub fn set_color(&mut self, color: Color) -> &mut Self {
        self.flag = (self.flag & 0b11111011) | color.as_u8();
        self
    }
    #[inline(always)]
    pub fn set_red(&mut self) -> &mut Self {
        self.flag = (self.flag & 0b11111011) | RED;
        self
    }
    #[inline(always)]
    pub fn set_black(&mut self) -> &mut Self {
        self.flag = (self.flag & 0b11111011) | BLACK;
        self
    }
    #[inline(always)]
    pub fn set_root(&mut self) -> &mut Self {
        self.flag = (self.flag & 0b11111000) | ROOT | BLACK;
        self
    }
    #[inline(always)]
    pub fn clear_root(&mut self) -> &mut Self {
        self.flag = self.flag & &!ROOT;
        self
    }
    #[inline(always)]
    pub fn rela(&self) -> u8 {
        self.flag & 0b11
    }
    #[inline(always)]
    pub fn color(&self) -> Color {
        Color::from(self.flag & 0b0100)
    }
}

pub fn toggle_rela(rela: u8) -> u8 {
    match rela {
        0 => 1,
        1 => 0,
        2 => 2,
        _ => unreachable!(),
    }
}
