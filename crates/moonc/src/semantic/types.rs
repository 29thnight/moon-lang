use serde::Serialize;

/// Moon type system representation.
///
/// Every expression and symbol has a `MoonType`. The type system enforces
/// null safety at compile time.

/// The core type enum.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum MoonType {
    /// Primitive types: Int, Float, Double, Bool, String, Long, Byte
    Primitive(PrimitiveKind),
    /// Unit (void)
    Unit,
    /// Nullable wrapper — `T?`
    Nullable(Box<MoonType>),
    /// A Moon-defined component type
    Component(String),
    /// A Moon-defined asset type
    Asset(String),
    /// A Moon-defined class
    Class(String),
    /// A Moon-defined enum
    Enum(String),
    /// An external C#/Unity type (not defined in Moon)
    External(String),
    /// Generic type application, e.g. `List<Int>`
    Generic(String, Vec<MoonType>),
    /// Error sentinel — used for error recovery
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum PrimitiveKind {
    Int,
    Float,
    Double,
    Bool,
    String,
    Long,
    Byte,
}

impl MoonType {
    /// Is this type nullable?
    pub fn is_nullable(&self) -> bool {
        matches!(self, MoonType::Nullable(_))
    }

    /// Get the non-null version (unwrap Nullable).
    pub fn non_null(&self) -> &MoonType {
        match self {
            MoonType::Nullable(inner) => inner,
            other => other,
        }
    }

    /// Make this type nullable.
    pub fn make_nullable(self) -> MoonType {
        if self.is_nullable() {
            self
        } else {
            MoonType::Nullable(Box::new(self))
        }
    }

    /// Is this a numeric type?
    pub fn is_numeric(&self) -> bool {
        matches!(
            self.non_null(),
            MoonType::Primitive(PrimitiveKind::Int)
                | MoonType::Primitive(PrimitiveKind::Float)
                | MoonType::Primitive(PrimitiveKind::Double)
                | MoonType::Primitive(PrimitiveKind::Long)
        )
    }

    /// Is this the error sentinel?
    pub fn is_error(&self) -> bool {
        matches!(self, MoonType::Error)
    }

    /// Check if `self` is assignable to `target`.
    pub fn is_assignable_to(&self, target: &MoonType) -> bool {
        if self.is_error() || target.is_error() {
            return true; // suppress cascading errors
        }
        if self == target {
            return true;
        }
        // null is assignable to any nullable type
        // A non-null T is assignable to T?
        if let MoonType::Nullable(inner) = target {
            if self.is_assignable_to(inner) {
                return true;
            }
        }
        // Numeric widening: Int → Float, Int → Double, Float → Double
        match (self.non_null(), target.non_null()) {
            (MoonType::Primitive(PrimitiveKind::Int), MoonType::Primitive(PrimitiveKind::Float)) => true,
            (MoonType::Primitive(PrimitiveKind::Int), MoonType::Primitive(PrimitiveKind::Double)) => true,
            (MoonType::Primitive(PrimitiveKind::Float), MoonType::Primitive(PrimitiveKind::Double)) => true,
            _ => false,
        }
    }

    /// Human-readable name.
    pub fn display_name(&self) -> String {
        match self {
            MoonType::Primitive(k) => match k {
                PrimitiveKind::Int => "Int".into(),
                PrimitiveKind::Float => "Float".into(),
                PrimitiveKind::Double => "Double".into(),
                PrimitiveKind::Bool => "Bool".into(),
                PrimitiveKind::String => "String".into(),
                PrimitiveKind::Long => "Long".into(),
                PrimitiveKind::Byte => "Byte".into(),
            },
            MoonType::Unit => "Unit".into(),
            MoonType::Nullable(inner) => format!("{}?", inner.display_name()),
            MoonType::Component(name) => name.clone(),
            MoonType::Asset(name) => name.clone(),
            MoonType::Class(name) => name.clone(),
            MoonType::Enum(name) => name.clone(),
            MoonType::External(name) => name.clone(),
            MoonType::Generic(name, args) => {
                let args_str: Vec<_> = args.iter().map(|a| a.display_name()).collect();
                format!("{}<{}>", name, args_str.join(", "))
            }
            MoonType::Error => "<error>".into(),
        }
    }
}

/// Resolve a type name string to a MoonType.
pub fn resolve_type_name(name: &str) -> MoonType {
    match name {
        "Int" => MoonType::Primitive(PrimitiveKind::Int),
        "Float" => MoonType::Primitive(PrimitiveKind::Float),
        "Double" => MoonType::Primitive(PrimitiveKind::Double),
        "Bool" => MoonType::Primitive(PrimitiveKind::Bool),
        "String" => MoonType::Primitive(PrimitiveKind::String),
        "Long" => MoonType::Primitive(PrimitiveKind::Long),
        "Byte" => MoonType::Primitive(PrimitiveKind::Byte),
        "Unit" => MoonType::Unit,
        _ => MoonType::External(name.to_string()),
    }
}
