use serde::Serialize;

/// PrSM type system representation.
///
/// Every expression and symbol has a `PrismType`. The type system enforces
/// null safety at compile time.

/// The core type enum.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum PrismType {
    /// Primitive types: Int, Float, Double, Bool, String, Long, Byte
    Primitive(PrimitiveKind),
    /// Unit (void)
    Unit,
    /// Nullable wrapper — `T?`
    Nullable(Box<PrismType>),
    /// A PrSM-defined component type
    Component(String),
    /// A PrSM-defined asset type
    Asset(String),
    /// A PrSM-defined class
    Class(String),
    /// A PrSM-defined enum
    Enum(String),
    /// An external C#/Unity type (not defined in PrSM)
    External(String),
    /// Generic type application, e.g. `List<Int>`
    Generic(String, Vec<PrismType>),
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

impl PrismType {
    /// Is this type nullable?
    pub fn is_nullable(&self) -> bool {
        matches!(self, PrismType::Nullable(_))
    }

    /// Get the non-null version (unwrap Nullable).
    pub fn non_null(&self) -> &PrismType {
        match self {
            PrismType::Nullable(inner) => inner,
            other => other,
        }
    }

    /// Make this type nullable.
    pub fn make_nullable(self) -> PrismType {
        if self.is_nullable() {
            self
        } else {
            PrismType::Nullable(Box::new(self))
        }
    }

    /// Is this a numeric type?
    pub fn is_numeric(&self) -> bool {
        matches!(
            self.non_null(),
            PrismType::Primitive(PrimitiveKind::Int)
                | PrismType::Primitive(PrimitiveKind::Float)
                | PrismType::Primitive(PrimitiveKind::Double)
                | PrismType::Primitive(PrimitiveKind::Long)
        )
    }

    /// Is this the error sentinel?
    pub fn is_error(&self) -> bool {
        matches!(self, PrismType::Error)
    }

    /// Check if `self` is assignable to `target`.
    pub fn is_assignable_to(&self, target: &PrismType) -> bool {
        if self.is_error() || target.is_error() {
            return true; // suppress cascading errors
        }
        if self == target {
            return true;
        }
        // Issue #20: a lambda literal trusts the explicit annotation
        // on the target. The semantic analyzer reports the lambda's
        // type as `External("lambda")` because PrSM does not yet
        // construct full function types from lambda parameters and
        // body. Treat the lambda as assignable to any function-shaped
        // target so the lang-4 spec example
        // `val callback: (Int) => Unit = { x => log(x) }` compiles.
        if let PrismType::External(name) = self {
            if name == "lambda" {
                return matches!(target, PrismType::External(_));
            }
        }
        // null is assignable to any nullable type
        // A non-null T is assignable to T?
        if let PrismType::Nullable(inner) = target {
            if self.is_assignable_to(inner) {
                return true;
            }
        }
        // Numeric widening: Int → Float, Int → Double, Float → Double
        match (self.non_null(), target.non_null()) {
            (PrismType::Primitive(PrimitiveKind::Int), PrismType::Primitive(PrimitiveKind::Float)) => true,
            (PrismType::Primitive(PrimitiveKind::Int), PrismType::Primitive(PrimitiveKind::Double)) => true,
            (PrismType::Primitive(PrimitiveKind::Float), PrismType::Primitive(PrimitiveKind::Double)) => true,
            _ => false,
        }
    }

    /// Human-readable name.
    pub fn display_name(&self) -> String {
        match self {
            PrismType::Primitive(k) => match k {
                PrimitiveKind::Int => "Int".into(),
                PrimitiveKind::Float => "Float".into(),
                PrimitiveKind::Double => "Double".into(),
                PrimitiveKind::Bool => "Bool".into(),
                PrimitiveKind::String => "String".into(),
                PrimitiveKind::Long => "Long".into(),
                PrimitiveKind::Byte => "Byte".into(),
            },
            PrismType::Unit => "Unit".into(),
            PrismType::Nullable(inner) => format!("{}?", inner.display_name()),
            PrismType::Component(name) => name.clone(),
            PrismType::Asset(name) => name.clone(),
            PrismType::Class(name) => name.clone(),
            PrismType::Enum(name) => name.clone(),
            PrismType::External(name) => name.clone(),
            PrismType::Generic(name, args) => {
                let args_str: Vec<_> = args.iter().map(|a| a.display_name()).collect();
                format!("{}<{}>", name, args_str.join(", "))
            }
            PrismType::Error => "<error>".into(),
        }
    }
}

/// Resolve a type name string to a PrismType.
pub fn resolve_type_name(name: &str) -> PrismType {
    match name {
        "Int" => PrismType::Primitive(PrimitiveKind::Int),
        "Float" => PrismType::Primitive(PrimitiveKind::Float),
        "Double" => PrismType::Primitive(PrimitiveKind::Double),
        "Bool" => PrismType::Primitive(PrimitiveKind::Bool),
        "String" => PrismType::Primitive(PrimitiveKind::String),
        "Long" => PrismType::Primitive(PrimitiveKind::Long),
        "Byte" => PrismType::Primitive(PrimitiveKind::Byte),
        "Unit" => PrismType::Unit,
        _ => PrismType::External(name.to_string()),
    }
}
