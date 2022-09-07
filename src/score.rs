use std::cmp::Ordering;

use owlchess::Color;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub enum Bound {
    Lower,
    Upper,
    Exact,
}

impl Default for Bound {
    #[inline]
    fn default() -> Self {
        Bound::Exact
    }
}

impl Bound {
    #[inline]
    pub fn inv(self) -> Self {
        match self {
            Self::Lower => Self::Upper,
            Self::Upper => Self::Lower,
            Self::Exact => Self::Exact,
        }
    }

    fn rel_side(self, side: Color) -> Self {
        match side {
            Color::White => self,
            Color::Black => self.inv(),
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub enum RelScore {
    Cp(i32),
    Mate { moves: u32, win: bool },
}

impl Default for RelScore {
    #[inline]
    fn default() -> Self {
        RelScore::Cp(0)
    }
}

impl RelScore {
    #[inline]
    pub fn inv(self) -> Self {
        match self {
            Self::Cp(x) => Self::Cp(-x),
            Self::Mate { moves, win } => Self::Mate { moves, win: !win },
        }
    }

    #[inline]
    pub fn abs_to(self, side: Color) -> AbsScore {
        match self {
            Self::Cp(val) => match side {
                Color::White => AbsScore::Cp(val),
                Color::Black => AbsScore::Cp(-val),
            },
            Self::Mate { moves, win } => AbsScore::Mate {
                moves,
                winner: if win { side } else { side.inv() },
            },
        }
    }

    fn as_cmp_tuple(&self) -> (i32, i64) {
        match *self {
            Self::Cp(val) => (0, val as i64),
            Self::Mate { moves, win: true } => (1, -(moves as i64)),
            Self::Mate { moves, win: false } => (-1, moves as i64),
        }
    }
}

impl PartialOrd for RelScore {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for RelScore {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_cmp_tuple().cmp(&other.as_cmp_tuple())
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub enum AbsScore {
    Cp(i32),
    Mate { moves: u32, winner: Color },
}

impl Default for AbsScore {
    #[inline]
    fn default() -> Self {
        AbsScore::Cp(0)
    }
}

impl AbsScore {
    #[inline]
    pub fn rel_to(self, side: Color) -> RelScore {
        match self {
            Self::Cp(val) => match side {
                Color::White => RelScore::Cp(val),
                Color::Black => RelScore::Cp(-val),
            },
            Self::Mate { moves, winner } => RelScore::Mate {
                moves,
                win: winner == side,
            },
        }
    }
}

impl PartialOrd for AbsScore {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for AbsScore {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.rel_to(Color::White).cmp(&other.rel_to(Color::White))
    }
}

#[derive(Copy, Clone, Default, PartialEq, Eq, Debug, Hash)]
pub struct BoundedRelScore {
    pub score: RelScore,
    pub bound: Bound,
}

impl BoundedRelScore {
    #[inline]
    pub fn inv(self) -> Self {
        Self {
            score: self.score.inv(),
            bound: self.bound.inv(),
        }
    }

    #[inline]
    pub fn abs_to(self, side: Color) -> BoundedAbsScore {
        BoundedAbsScore {
            score: self.score.abs_to(side),
            bound: self.bound.rel_side(side),
        }
    }
}

#[derive(Copy, Clone, Default, PartialEq, Eq, Debug, Hash)]
pub struct BoundedAbsScore {
    pub score: AbsScore,
    pub bound: Bound,
}

impl BoundedAbsScore {
    #[inline]
    pub fn rel_to(self, side: Color) -> BoundedRelScore {
        BoundedRelScore {
            score: self.score.rel_to(side),
            bound: self.bound.rel_side(side),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert() {
        for rel in [
            RelScore::Cp(-100),
            RelScore::Cp(280),
            RelScore::Cp(0),
            RelScore::Cp(-410),
            RelScore::Mate {
                moves: 2,
                win: true,
            },
            RelScore::Mate {
                moves: 0,
                win: true,
            },
            RelScore::Mate {
                moves: 5,
                win: true,
            },
            RelScore::Mate {
                moves: 3,
                win: false,
            },
            RelScore::Mate {
                moves: 0,
                win: false,
            },
            RelScore::Mate {
                moves: 9,
                win: false,
            },
        ] {
            let abs_white = rel.abs_to(Color::White);
            let abs_black = rel.abs_to(Color::Black);
            assert_eq!(abs_white.rel_to(Color::White), rel);
            assert_eq!(abs_black.rel_to(Color::Black), rel);
            assert_eq!(abs_white.rel_to(Color::Black), rel.inv());
            assert_eq!(abs_black.rel_to(Color::White), rel.inv());
        }
    }

    #[test]
    fn test_sort_rel() {
        let mut src = [
            RelScore::Cp(-100),
            RelScore::Cp(280),
            RelScore::Cp(0),
            RelScore::Cp(-410),
            RelScore::Mate {
                moves: 2,
                win: true,
            },
            RelScore::Mate {
                moves: 0,
                win: true,
            },
            RelScore::Mate {
                moves: 5,
                win: true,
            },
            RelScore::Mate {
                moves: 3,
                win: false,
            },
            RelScore::Mate {
                moves: 0,
                win: false,
            },
            RelScore::Mate {
                moves: 9,
                win: false,
            },
        ];
        let res = [
            RelScore::Mate {
                moves: 0,
                win: false,
            },
            RelScore::Mate {
                moves: 3,
                win: false,
            },
            RelScore::Mate {
                moves: 9,
                win: false,
            },
            RelScore::Cp(-410),
            RelScore::Cp(-100),
            RelScore::Cp(0),
            RelScore::Cp(280),
            RelScore::Mate {
                moves: 5,
                win: true,
            },
            RelScore::Mate {
                moves: 2,
                win: true,
            },
            RelScore::Mate {
                moves: 0,
                win: true,
            },
        ];
        src.sort();
        assert_eq!(src, res);
    }

    #[test]
    fn test_sort_abs() {
        let mut src = [
            AbsScore::Cp(-100),
            AbsScore::Cp(280),
            AbsScore::Cp(0),
            AbsScore::Cp(-410),
            AbsScore::Mate {
                moves: 2,
                winner: Color::White,
            },
            AbsScore::Mate {
                moves: 0,
                winner: Color::White,
            },
            AbsScore::Mate {
                moves: 5,
                winner: Color::White,
            },
            AbsScore::Mate {
                moves: 3,
                winner: Color::Black,
            },
            AbsScore::Mate {
                moves: 0,
                winner: Color::Black,
            },
            AbsScore::Mate {
                moves: 9,
                winner: Color::Black,
            },
        ];
        let res = [
            AbsScore::Mate {
                moves: 0,
                winner: Color::Black,
            },
            AbsScore::Mate {
                moves: 3,
                winner: Color::Black,
            },
            AbsScore::Mate {
                moves: 9,
                winner: Color::Black,
            },
            AbsScore::Cp(-410),
            AbsScore::Cp(-100),
            AbsScore::Cp(0),
            AbsScore::Cp(280),
            AbsScore::Mate {
                moves: 5,
                winner: Color::White,
            },
            AbsScore::Mate {
                moves: 2,
                winner: Color::White,
            },
            AbsScore::Mate {
                moves: 0,
                winner: Color::White,
            },
        ];
        src.sort();
        assert_eq!(src, res);
    }
}
