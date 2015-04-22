use std::ops;
use std::num::ToPrimitive;
use std::cmp::Ordering;
use std::iter::IntoIterator;

use index::Index;

#[derive(Clone, Debug, PartialOrd, PartialEq)]
pub enum Value {
    Bool(bool),
    String(String),
    Float(f64),
    Tuple(Tuple),
    Relation(Relation),
}
pub type Tuple = Vec<Value>;
pub type Relation = Index<Vec<Value>>; // a set of tuples
pub type Id = String; // TODO use uuid?

impl Ord for Value {
    fn cmp(&self, other: &Value) -> Ordering {
        self.partial_cmp(other).unwrap() // TODO this will panic on NaN
    }
}

impl Eq for Value {} // TODO this is unsafe for NaN

impl ops::Index<usize> for Value {
    type Output = Value;

    fn index(&self, index: usize) -> &Value {
        match *self {
            Value::Tuple(ref tuple) => tuple.index(index),
            _ => panic!("Indexing a non-tuple value"),
        }
    }
}

impl ToPrimitive for Value {
    fn to_f64(&self) -> Option<f64> {
        match *self {
            Value::Float(ref float) => float.to_f64(),
            _ => None,
        }
    }
    fn to_i64(&self) -> Option<i64> {
        match *self {
            Value::Float(ref float) => float.to_i64(),
            _ => None,
        }
    }
    fn to_u64(&self) -> Option<u64> {
        match *self {
            Value::Float(ref float) => float.to_u64(),
            _ => None,
        }
    }
}

impl Value {
    pub fn as_str(&self) -> &str {
        match *self {
            Value::String(ref string) => &*string,
            _ => panic!("Not a string: {:?}", self),
        }
    }

    pub fn as_slice(&self) -> &[Value] {
        match *self {
            Value::Tuple(ref tuple) => &*tuple,
            _ => panic!("Not a tuple: {:?}", self),
        }
    }
}

// Convenient hacks for writing tests
// Do not use in production code

pub trait ToValue {
    fn to_value(self) -> Value;
}

pub trait ToTuple {
    fn to_tuple(self) -> Tuple;
}

pub trait ToRelation {
    fn to_relation(self) -> Relation;
}

impl ToValue for Value {
    fn to_value(self) -> Value {
        self
    }
}

impl ToValue for bool {
    fn to_value(self) -> Value {
        Value::Bool(self)
    }
}

impl<'a> ToValue for &'a str {
    fn to_value(self) -> Value {
        Value::String(self.to_string())
    }
}

impl ToValue for String {
    fn to_value(self) -> Value {
        Value::String(self)
    }
}

impl ToValue for f64 {
    fn to_value(self) -> Value {
        Value::Float(self)
    }
}

impl ToValue for i32 {
     fn to_value(self) -> Value {
        Value::Float(self as f64)
    }
}

impl ToValue for i64 {
     fn to_value(self) -> Value {
        Value::Float(self as f64)
    }
}

impl ToValue for Tuple {
    fn to_value(self) -> Value {
        Value::Tuple(self)
    }

}

impl ToValue for usize {
    fn to_value(self) -> Value {
        Value::Float(self.to_f64().unwrap())
    }
}

// impl<T: ToTuple> ToValue for T {
//     fn to_value(self) -> Value {
//         Value::Tuple(self.to_tuple())
//     }
// }

// impl<T: ToRelation> ToValue for T where T: !ToTuple {
//     fn to_value(self) -> Value {
//         Value::Relation(self.to_relation())
//     }
// }

impl<A: ToValue> ToTuple for (A,) {
    fn to_tuple(self) -> Vec<Value> {
        let (a,) = self;
        vec![a.to_value()]
    }
}

impl<A: ToValue, B: ToValue> ToTuple for (A,B) {
    fn to_tuple(self) -> Vec<Value> {
        let (a,b) = self;
        vec![a.to_value(), b.to_value()]
    }
}

impl<A: ToValue, B: ToValue, C: ToValue> ToTuple for (A,B,C) {
    fn to_tuple(self) -> Vec<Value> {
        let (a,b,c) = self;
        vec![a.to_value(), b.to_value(), c.to_value()]
    }
}

impl<A: ToValue, B: ToValue, C: ToValue, D: ToValue> ToTuple for (A,B,C,D) {
    fn to_tuple(self) -> Vec<Value> {
        let (a,b,c,d) = self;
        vec![a.to_value(), b.to_value(), c.to_value(), d.to_value()]
    }
}

impl<A: ToValue, B: ToValue, C: ToValue, D: ToValue, E: ToValue> ToTuple for (A,B,C,D,E) {
    fn to_tuple(self) -> Vec<Value> {
        let (a,b,c,d,e) = self;
        vec![a.to_value(), b.to_value(), c.to_value(), d.to_value(), e.to_value()]
    }
}

impl<T: ToTuple> ToRelation for Vec<T> {
    fn to_relation(self) -> Relation {
        self.into_iter().map(|t| t.to_tuple()).collect()
    }
}
