use std::fmt::Display;

use konst::const_panic::{PanicFmt, PanicVal};

#[allow(unused)]
#[derive(Debug, Copy, Clone, Hash)]
#[derive_const(PartialEq, Eq)]
pub enum Comment {
    Line(Option<DocStyle>),
    Block {
        style: Option<DocStyle>,
        terminated: bool,
    },
}

impl PanicFmt for Comment {
    type This = Self;

    type Kind = konst::const_panic::IsCustomType;

    const PV_COUNT: usize = 1;
}

impl Comment {
    pub const fn to_panicvals(
        self,
        _: konst::const_panic::FmtArg,
    ) -> [PanicVal<'static>; Self::PV_COUNT] {
        [PanicVal::write_str(self.as_printable_str())]
    }
}

impl Comment {
    pub const fn as_printable_str(&self) -> &'static str {
        match self {
            Self::Line(Some(s)) => match s {
                DocStyle::Outer => "OuterLine",
                DocStyle::Inner => "InnerLine",
            },
            Self::Line(None) => "Line",
            Self::Block {
                style: Some(s),
                terminated: true,
            } => match s {
                DocStyle::Outer => "OuterBlockTerminated",
                DocStyle::Inner => "InnerBlockTerminated",
            },
            Self::Block {
                style: Some(s),
                terminated: false,
            } => match s {
                DocStyle::Outer => "OuterBlockUnTerminated",
                DocStyle::Inner => "InnerBlockUnTerminated",
            },
            Self::Block {
                style: None,
                terminated,
            } => match terminated {
                true => "BlockTerminated",
                false => "BlockUnterminated",
            },
        }
    }
}

impl Display for Comment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_printable_str())
    }
}

#[allow(unused)]
#[derive(Debug, Copy, Hash)]
#[derive_const(Clone, PartialEq, Eq)]
pub enum DocStyle {
    Outer,
    Inner,
}

impl Display for DocStyle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Outer => write!(f, "Outer"),
            Self::Inner => write!(f, "Inner"),
        }
    }
}
