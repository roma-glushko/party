//! P language type system with subtyping rules.

use std::fmt;

/// Resolved type in the P type system.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PResolvedType {
    Bool,
    Int,
    Float,
    String,
    Event,
    Machine,
    Any,
    Null,
    Data,
    Seq(Box<PResolvedType>),
    Set(Box<PResolvedType>),
    Map(Box<PResolvedType>, Box<PResolvedType>),
    Tuple(Vec<PResolvedType>),
    NamedTuple(Vec<(std::string::String, PResolvedType)>),
    Enum(std::string::String),
    TypeDef(std::string::String, Box<PResolvedType>),
    Foreign(std::string::String),
    /// Machine interface type (for permission checking)
    Permission(std::string::String),
    /// Void (no value)
    Void,
}

impl PResolvedType {
    /// Check if a value of type `other` can be assigned to a variable of type `self`.
    pub fn is_assignable_from(&self, other: &PResolvedType) -> bool {
        let me = self.canonicalize();
        let them = other.canonicalize();

        match (&me, &them) {
            // Any accepts everything
            (PResolvedType::Any, _) => true,

            // Null is special — accepted by reference-like types
            (_, PResolvedType::Null) => matches!(
                me,
                PResolvedType::Any
                    | PResolvedType::Machine
                    | PResolvedType::Event
                    | PResolvedType::Null
                    | PResolvedType::Permission(_)
                    | PResolvedType::Foreign(_)
                    | PResolvedType::Data
            ),

            // Machine accepts machine, null, and permission types
            (PResolvedType::Machine, PResolvedType::Machine) => true,
            (PResolvedType::Machine, PResolvedType::Permission(_)) => true,

            // Event accepts event
            (PResolvedType::Event, PResolvedType::Event) => true,

            // Primitives: exact match only
            (PResolvedType::Bool, PResolvedType::Bool) => true,
            (PResolvedType::Int, PResolvedType::Int) => true,
            (PResolvedType::Float, PResolvedType::Float) => true,
            (PResolvedType::String, PResolvedType::String) => true,

            // Collections: `any` element type accepts any collection of same kind
            (PResolvedType::Seq(a), PResolvedType::Seq(b)) => a.is_assignable_from(b),
            (PResolvedType::Set(a), PResolvedType::Set(b)) => a.is_assignable_from(b),
            (PResolvedType::Map(k1, v1), PResolvedType::Map(k2, v2)) => {
                (k1.is_assignable_from(k2)) && (v1.is_assignable_from(v2))
            }

            // Tuples: same arity, pairwise same type
            (PResolvedType::Tuple(a), PResolvedType::Tuple(b)) => {
                a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| x.is_same_type(y))
            }

            // Named tuples: same arity, same names, pairwise assignable
            (PResolvedType::NamedTuple(a), PResolvedType::NamedTuple(b)) => {
                a.len() == b.len()
                    && a.iter()
                        .zip(b.iter())
                        .all(|((n1, t1), (n2, t2))| n1 == n2 && t1.is_assignable_from(t2))
            }

            // Enums: nominal (same enum name, also match foreign)
            (PResolvedType::Enum(a), PResolvedType::Enum(b)) => a == b,
            (PResolvedType::Enum(a), PResolvedType::Foreign(b)) => a == b,
            (PResolvedType::Foreign(a), PResolvedType::Enum(b)) => a == b,

            // Permission types (interface/machine references)
            (PResolvedType::Permission(_), PResolvedType::Permission(_)) => true,
            (PResolvedType::Permission(_), PResolvedType::Machine) => true,

            // Data accepts anything without permissions
            (PResolvedType::Data, _) => true,

            // Foreign types: nominal, also match Permission types
            (PResolvedType::Foreign(a), PResolvedType::Foreign(b)) => a == b,
            (PResolvedType::Foreign(a), PResolvedType::Permission(b)) => a == b,
            (PResolvedType::Permission(a), PResolvedType::Foreign(b)) => a == b,
            // Machine accepts foreign/permission types (interface references)
            (PResolvedType::Machine, PResolvedType::Foreign(_)) => true,

            _ => false,
        }
    }

    /// Bidirectional assignability (both directions work).
    pub fn is_same_type(&self, other: &PResolvedType) -> bool {
        self.is_assignable_from(other) && other.is_assignable_from(self)
    }

    /// Strip typedefs to get the underlying type.
    pub fn canonicalize(&self) -> PResolvedType {
        match self {
            PResolvedType::TypeDef(_, inner) => inner.canonicalize(),
            other => other.clone(),
        }
    }

    /// Is this a collection type?
    pub fn is_collection(&self) -> bool {
        matches!(
            self.canonicalize(),
            PResolvedType::Seq(_) | PResolvedType::Set(_) | PResolvedType::Map(_, _)
        )
    }

    /// Is this a numeric type?
    pub fn is_numeric(&self) -> bool {
        matches!(self.canonicalize(), PResolvedType::Int | PResolvedType::Float)
    }

    /// Get the default value type representation.
    pub fn default_assignable(&self) -> bool {
        // All types have defaults
        true
    }
}

impl fmt::Display for PResolvedType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PResolvedType::Bool => write!(f, "bool"),
            PResolvedType::Int => write!(f, "int"),
            PResolvedType::Float => write!(f, "float"),
            PResolvedType::String => write!(f, "string"),
            PResolvedType::Event => write!(f, "event"),
            PResolvedType::Machine => write!(f, "machine"),
            PResolvedType::Any => write!(f, "any"),
            PResolvedType::Null => write!(f, "null"),
            PResolvedType::Data => write!(f, "data"),
            PResolvedType::Void => write!(f, "void"),
            PResolvedType::Seq(t) => write!(f, "seq[{t}]"),
            PResolvedType::Set(t) => write!(f, "set[{t}]"),
            PResolvedType::Map(k, v) => write!(f, "map[{k}, {v}]"),
            PResolvedType::Tuple(ts) => {
                write!(f, "(")?;
                for (i, t) in ts.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{t}")?;
                }
                write!(f, ")")
            }
            PResolvedType::NamedTuple(fields) => {
                write!(f, "(")?;
                for (i, (name, ty)) in fields.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{name}: {ty}")?;
                }
                write!(f, ")")
            }
            PResolvedType::Enum(name) => write!(f, "{name}"),
            PResolvedType::TypeDef(name, _) => write!(f, "{name}"),
            PResolvedType::Foreign(name) => write!(f, "{name}"),
            PResolvedType::Permission(name) => write!(f, "{name}"),
        }
    }
}
