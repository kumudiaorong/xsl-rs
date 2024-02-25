use core::fmt::Display;

#[derive(Copy, Clone)]
pub struct Flag {
    pub flag: u8,
}
#[derive(Clone, Copy, PartialEq)]
pub enum Rela {
    LEFT,
    RIGHT,
    PARENT,
}
impl From<u8> for Rela {
    fn from(flag: u8) -> Self {
        match flag {
            0b00 => Rela::LEFT,
            0b01 => Rela::RIGHT,
            0b10 => Rela::PARENT,
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
            Rela::PARENT => write!(f, "P"),
        }
    }
}
impl Rela {
    pub fn toggle(&self) -> Self {
        match self {
            Rela::LEFT => Rela::RIGHT,
            Rela::RIGHT => Rela::LEFT,
            Rela::PARENT => Rela::PARENT,
        }
    }
}
#[repr(u8)]
#[derive(Clone, Copy, PartialEq)]
pub enum Color {
    RED,
    BLACK,
    ROOT,
}
impl From<u8> for Color {
    fn from(flag: u8) -> Self {
        match flag {
            0b000 => Color::RED,
            0b100 => Color::BLACK,
            0b1000 => Color::ROOT,
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
            Color::ROOT => write!(f, "ROOT"),
        }
    }
}
impl Color {
    pub fn toggle(&self) -> Self {
        match self {
            Color::RED => Color::BLACK,
            Color::BLACK => Color::RED,
            Color::ROOT => Color::ROOT,
        }
    }
}
impl Flag {
    pub fn new() -> Self {
        Flag { flag: 0 }
    }
    pub fn set(&mut self, flag: u8) {
        self.flag = flag;
    }
    pub fn clear(&mut self) {
        self.flag = 0;
    }
    pub fn is_red(&self) -> bool {
        self.flag & 0b1100 == Color::RED.into()
    }
    pub fn is_black(&self) -> bool {
        self.flag & 0b1100 == Color::BLACK.into()
    }
    pub fn is_root(&self) -> bool {
        self.flag & 0b1100 == Color::ROOT.into()
    }
    pub fn is_left(&self) -> bool {
        self.flag & 0b11 == Rela::LEFT.into()
    }
    pub fn is_right(&self) -> bool {
        self.flag & 0b11 == Rela::RIGHT.into()
    }
    pub fn set_rela(&mut self, rela: Rela) -> &mut Self {
        self.flag = (self.flag & 0b11111100) | Into::<u8>::into(rela);
        self
    }
    pub fn set_color(&mut self, color: Color) -> &mut Self {
        self.flag = (self.flag & 0b11110000) | Into::<u8>::into(color);
        self
    }
    pub fn set_red(&mut self) -> &mut Self {
        self.flag = (self.flag & 0b11110011) | Into::<u8>::into(Color::RED);
        self
    }
    pub fn set_black(&mut self) -> &mut Self {
        self.flag = (self.flag & 0b11110011) | Into::<u8>::into(Color::BLACK);
        self
    }
    pub fn set_root(&mut self) -> &mut Self {
        self.flag = (self.flag & 0b11110011) | Into::<u8>::into(Color::ROOT);
        self
    }
    pub fn set_left(&mut self) -> &mut Self {
        self.flag = (self.flag & 0b11111100) | Into::<u8>::into(Rela::LEFT);
        self
    }
    pub fn set_right(&mut self) -> &mut Self {
        self.flag = (self.flag & 0b11111100) | Into::<u8>::into(Rela::RIGHT);
        self
    }
    pub fn rela(&self) -> Rela {
        Rela::from(self.flag & 0b11)
    }
    pub fn color(&self) -> Color {
        Color::from(self.flag & 0b1100)
    }
}
