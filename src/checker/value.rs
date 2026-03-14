//! Runtime values for P programs. All values have clone (copy) semantics.

use std::collections::BTreeMap;
use std::fmt;

/// Runtime value in a P program.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PValue {
    Null,
    Bool(bool),
    Int(i64),
    Float(OrderedFloat),
    String(String),
    MachineRef(usize), // machine ID
    EventId(String),   // event name
    EnumVal(String, String), // enum type name, element name
    Seq(Vec<PValue>),
    Set(Vec<PValue>), // sorted vec for determinism
    Map(BTreeMap<PValue, PValue>),
    Tuple(Vec<PValue>),
    NamedTuple(Vec<(String, PValue)>),
}

/// Wrapper for f64 that implements Ord (for use in collections).
#[derive(Debug, Clone, Copy)]
pub struct OrderedFloat(pub f64);

impl PartialEq for OrderedFloat {
    fn eq(&self, other: &Self) -> bool {
        self.0.to_bits() == other.0.to_bits()
    }
}
impl Eq for OrderedFloat {}

impl PartialOrd for OrderedFloat {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for OrderedFloat {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.total_cmp(&other.0)
    }
}
impl std::hash::Hash for OrderedFloat {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.to_bits().hash(state);
    }
}

impl PValue {
    pub fn default_int() -> Self { PValue::Int(0) }
    pub fn default_bool() -> Self { PValue::Bool(false) }
    pub fn default_float() -> Self { PValue::Float(OrderedFloat(0.0)) }
    pub fn default_string() -> Self { PValue::String(String::new()) }

    pub fn as_bool(&self) -> Option<bool> {
        match self { PValue::Bool(b) => Some(*b), _ => None }
    }
    pub fn as_int(&self) -> Option<i64> {
        match self { PValue::Int(i) => Some(*i), _ => None }
    }
    pub fn as_float(&self) -> Option<f64> {
        match self { PValue::Float(f) => Some(f.0), _ => None }
    }
    pub fn as_machine_ref(&self) -> Option<usize> {
        match self { PValue::MachineRef(id) => Some(*id), _ => None }
    }

    pub fn is_null(&self) -> bool {
        matches!(self, PValue::Null)
    }

    pub fn to_bool(&self) -> bool {
        match self {
            PValue::Bool(b) => *b,
            PValue::Int(i) => *i != 0,
            PValue::Null => false,
            _ => true,
        }
    }
}

impl fmt::Display for PValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PValue::Null => write!(f, "null"),
            PValue::Bool(b) => write!(f, "{b}"),
            PValue::Int(i) => write!(f, "{i}"),
            PValue::Float(v) => write!(f, "{}", v.0),
            PValue::String(s) => write!(f, "{s}"),
            PValue::MachineRef(id) => write!(f, "machine#{id}"),
            PValue::EventId(name) => write!(f, "event:{name}"),
            PValue::EnumVal(_, elem) => write!(f, "{elem}"),
            PValue::Seq(items) => {
                write!(f, "[")?;
                for (i, v) in items.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{v}")?;
                }
                write!(f, "]")
            }
            PValue::Set(items) => {
                write!(f, "{{")?;
                for (i, v) in items.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{v}")?;
                }
                write!(f, "}}")
            }
            PValue::Map(items) => {
                write!(f, "{{")?;
                for (i, (k, v)) in items.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{k} -> {v}")?;
                }
                write!(f, "}}")
            }
            PValue::Tuple(fields) => {
                write!(f, "(")?;
                for (i, v) in fields.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{v}")?;
                }
                write!(f, ")")
            }
            PValue::NamedTuple(fields) => {
                write!(f, "(")?;
                for (i, (name, v)) in fields.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{name} = {v}")?;
                }
                write!(f, ")")
            }
        }
    }
}
