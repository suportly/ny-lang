use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NyType {
    I8,
    I16,
    I32,
    I64,
    I128,
    U8,
    U16,
    U32,
    U64,
    U128,
    F32,
    F64,
    Bool,
    Unit,
    Function {
        params: Vec<NyType>,
        ret: Box<NyType>,
    },
}

impl NyType {
    pub fn is_integer(&self) -> bool {
        matches!(
            self,
            NyType::I8
                | NyType::I16
                | NyType::I32
                | NyType::I64
                | NyType::I128
                | NyType::U8
                | NyType::U16
                | NyType::U32
                | NyType::U64
                | NyType::U128
        )
    }

    pub fn is_signed(&self) -> bool {
        matches!(
            self,
            NyType::I8 | NyType::I16 | NyType::I32 | NyType::I64 | NyType::I128
        )
    }

    pub fn is_float(&self) -> bool {
        matches!(self, NyType::F32 | NyType::F64)
    }

    pub fn is_numeric(&self) -> bool {
        self.is_integer() || self.is_float()
    }

    pub fn from_name(name: &str) -> Option<NyType> {
        match name {
            "i8" => Some(NyType::I8),
            "i16" => Some(NyType::I16),
            "i32" => Some(NyType::I32),
            "i64" => Some(NyType::I64),
            "i128" => Some(NyType::I128),
            "u8" => Some(NyType::U8),
            "u16" => Some(NyType::U16),
            "u32" => Some(NyType::U32),
            "u64" => Some(NyType::U64),
            "u128" => Some(NyType::U128),
            "f32" => Some(NyType::F32),
            "f64" => Some(NyType::F64),
            "bool" => Some(NyType::Bool),
            _ => None,
        }
    }
}

impl fmt::Display for NyType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NyType::I8 => write!(f, "i8"),
            NyType::I16 => write!(f, "i16"),
            NyType::I32 => write!(f, "i32"),
            NyType::I64 => write!(f, "i64"),
            NyType::I128 => write!(f, "i128"),
            NyType::U8 => write!(f, "u8"),
            NyType::U16 => write!(f, "u16"),
            NyType::U32 => write!(f, "u32"),
            NyType::U64 => write!(f, "u64"),
            NyType::U128 => write!(f, "u128"),
            NyType::F32 => write!(f, "f32"),
            NyType::F64 => write!(f, "f64"),
            NyType::Bool => write!(f, "bool"),
            NyType::Unit => write!(f, "()"),
            NyType::Function { params, ret } => {
                write!(f, "fn(")?;
                for (i, p) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", p)?;
                }
                write!(f, ") -> {}", ret)
            }
        }
    }
}
