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
    Str,
    Function {
        params: Vec<NyType>,
        ret: Box<NyType>,
    },
    Array {
        elem: Box<NyType>,
        size: usize,
    },
    Struct {
        name: String,
        fields: Vec<(String, NyType)>,
    },
    Pointer(Box<NyType>),
    Enum {
        name: String,
        variants: Vec<(String, Vec<NyType>)>,
    },
    Tuple(Vec<NyType>),
    Slice(Box<NyType>),
    /// SIMD vector type: elem type + lane count
    Simd {
        elem: Box<NyType>,
        lanes: u32,
    },
    Vec(Box<NyType>),
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

    pub fn is_array(&self) -> bool {
        matches!(self, NyType::Array { .. })
    }

    pub fn is_struct(&self) -> bool {
        matches!(self, NyType::Struct { .. })
    }

    pub fn is_pointer(&self) -> bool {
        matches!(self, NyType::Pointer(_))
    }

    pub fn is_enum(&self) -> bool {
        matches!(self, NyType::Enum { .. })
    }

    pub fn is_tuple(&self) -> bool {
        matches!(self, NyType::Tuple(_))
    }

    pub fn is_slice(&self) -> bool {
        matches!(self, NyType::Slice(_))
    }

    pub fn is_vec(&self) -> bool {
        matches!(self, NyType::Vec(_))
    }

    pub fn is_simd(&self) -> bool {
        matches!(self, NyType::Simd { .. })
    }

    pub fn variant_index(&self, variant: &str) -> Option<usize> {
        match self {
            NyType::Enum { variants, .. } => variants.iter().position(|(name, _)| name == variant),
            _ => None,
        }
    }

    pub fn variant_payload(&self, variant: &str) -> Option<&Vec<NyType>> {
        match self {
            NyType::Enum { variants, .. } => variants
                .iter()
                .find(|(name, _)| name == variant)
                .map(|(_, payload)| payload),
            _ => None,
        }
    }

    pub fn elem_type(&self) -> Option<&NyType> {
        match self {
            NyType::Array { elem, .. } => Some(elem),
            NyType::Pointer(inner) => Some(inner),
            _ => None,
        }
    }

    pub fn array_size(&self) -> Option<usize> {
        match self {
            NyType::Array { size, .. } => Some(*size),
            _ => None,
        }
    }

    pub fn field_type(&self, field_name: &str) -> Option<&NyType> {
        match self {
            NyType::Struct { fields, .. } => {
                fields.iter().find(|(n, _)| n == field_name).map(|(_, t)| t)
            }
            _ => None,
        }
    }

    pub fn struct_name(&self) -> Option<&str> {
        match self {
            NyType::Struct { name, .. } => Some(name),
            _ => None,
        }
    }

    pub fn pointee(&self) -> Option<&NyType> {
        match self {
            NyType::Pointer(inner) => Some(inner),
            _ => None,
        }
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
            "str" => Some(NyType::Str),
            // SIMD types
            "f32x4" => Some(NyType::Simd {
                elem: Box::new(NyType::F32),
                lanes: 4,
            }),
            "f32x8" => Some(NyType::Simd {
                elem: Box::new(NyType::F32),
                lanes: 8,
            }),
            "f64x2" => Some(NyType::Simd {
                elem: Box::new(NyType::F64),
                lanes: 2,
            }),
            "f64x4" => Some(NyType::Simd {
                elem: Box::new(NyType::F64),
                lanes: 4,
            }),
            "i32x4" => Some(NyType::Simd {
                elem: Box::new(NyType::I32),
                lanes: 4,
            }),
            "i32x8" => Some(NyType::Simd {
                elem: Box::new(NyType::I32),
                lanes: 8,
            }),
            _ => {
                // Check for Vec<T> pattern
                if let Some(inner) = name.strip_prefix("Vec<").and_then(|s| s.strip_suffix('>')) {
                    if let Some(elem_ty) = NyType::from_name(inner) {
                        return Some(NyType::Vec(Box::new(elem_ty)));
                    }
                    // Also try to match struct names embedded in Vec
                    return Some(NyType::Vec(Box::new(NyType::Str))); // fallback for unknown
                }
                None
            }
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
            NyType::Str => write!(f, "str"),
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
            NyType::Array { elem, size } => write!(f, "[{}]{}", size, elem),
            NyType::Struct { name, .. } => write!(f, "{}", name),
            NyType::Pointer(inner) => write!(f, "*{}", inner),
            NyType::Enum { name, .. } => write!(f, "{name}"),
            NyType::Simd { elem, lanes } => write!(f, "{}x{}", elem, lanes),
            NyType::Slice(elem) => write!(f, "[]{}", elem),
            NyType::Vec(elem) => write!(f, "Vec<{}>", elem),
            NyType::Tuple(elems) => {
                write!(f, "(")?;
                for (i, e) in elems.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", e)?;
                }
                write!(f, ")")
            }
        }
    }
}
