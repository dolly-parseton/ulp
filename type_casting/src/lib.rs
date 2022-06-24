use serde_json::Number;
use serde_json::Value;
use std::collections::BTreeMap;
use std::error::Error;
pub use types::Types;

mod types;

#[cfg(test)]
#[macro_use]
extern crate serde_json;
//

pub fn merge(left: &mut Types, right: Types) {
    use Types::*;
    // Complex
    match (&mut *left, right) {
        (Null, Null) => *left = Null,
        (Bool, Bool) => *left = Bool,
        (Int, Int) => *left = Int,
        (Float, Float) => *left = Float,
        (IPv4, IPv4) => *left = IPv4,
        (IPv6, IPv6) => *left = IPv6,
        (Date, Date) => *left = Date,
        (Str, Str) => *left = Str,
        //
        (Object(a), Object(mut b)) => {
            let mut keys = a.keys().cloned().collect::<Vec<_>>();
            let mut b_keys = b.keys().cloned().collect::<Vec<_>>();
            keys.append(&mut b_keys);
            keys.dedup();
            for key in keys {
                match (a.remove(&key), b.remove(&key)) {
                    (Some(mut a_value), Some(b_value)) => {
                        merge(&mut a_value, b_value);
                        a.insert(key, a_value);
                    }
                    (None, Some(value)) | (Some(value), None) => {
                        a.insert(key, value);
                    }
                    _ => (),
                }
                // }
            }
            return;
        }
        (List(ref mut a), List(mut b)) => {
            let mut keys = a.keys().cloned().collect::<Vec<_>>();
            let mut b_keys = b.keys().cloned().collect::<Vec<_>>();
            keys.append(&mut b_keys);
            keys.dedup();
            for key in keys {
                // if let Some(mut a_value) = a.remove(&key) {
                match (a.remove(&key), b.remove(&key)) {
                    (Some(mut a_value), Some(b_value)) => {
                        merge(&mut a_value, b_value);
                        a.insert(key, a_value);
                    }
                    (None, Some(value)) | (Some(value), None) => {
                        a.insert(key, value);
                    }
                    _ => (),
                }
                // }
            }
            return;
        }
        //
        (Object(ref mut a), List(ref mut b)) => {
            // Each list entry merged into Object using i as key.
            let keys = b.keys().cloned().collect::<Vec<_>>();
            for key in keys {
                if let Some(mut a_value) = a.remove(&key.to_string()) {
                    match b.remove(&key) {
                        Some(b_value) => {
                            merge(&mut a_value, b_value);
                            a.insert(key.to_string(), a_value);
                        }
                        None => {
                            a.insert(key.to_string(), a_value);
                        }
                    }
                }
            }
            return;
        }
        //
        (List(ref mut a), b) => {
            match a.remove(&0) {
                Some(mut a_value) => {
                    merge(&mut a_value, b);
                    a.insert(0, a_value);
                }
                None => (),
            };
            return;
        }
        //
        (Null, b) => *left = b,
        //
        (Bool, Null) => *left = Bool,
        (Bool, Int) => *left = Int,
        (Bool, Float) => *left = Float,
        (Bool, Str) => *left = Str,
        (Bool, IPv4) => *left = Str,
        (Bool, IPv6) => *left = Str,
        (Bool, Date) => *left = Str,
        (Bool, b) => *left = b,
        //
        (Int, Null) => *left = Int,
        (Int, Bool) => *left = Int,
        (Int, Float) => *left = Float,
        (Int, Str) => *left = Str,
        (Int, IPv4) => *left = Str,
        (Int, IPv6) => *left = Str,
        (Int, Date) => *left = Str,
        (Int, b) => *left = b,
        //
        (Float, Null) => *left = Float,
        (Float, Bool) => *left = Float,
        (Float, Int) => *left = Float,
        (Float, Str) => *left = Str,
        (Float, IPv4) => *left = Str,
        (Float, IPv6) => *left = Float,
        (Float, Date) => *left = Float,
        (Float, b) => *left = b,
        //
        (IPv4, Null) => *left = IPv4,
        (IPv4, Bool) => *left = Str,
        (IPv4, Int) => *left = Str,
        (IPv4, Float) => *left = Str,
        (IPv4, IPv6) => *left = Str,
        (IPv4, b) => *left = b,
        //
        (IPv6, Null) => *left = IPv6,
        (IPv6, Bool) => *left = Str,
        (IPv6, Int) => *left = Str,
        (IPv6, Float) => *left = Str,
        (IPv6, IPv4) => *left = Str,
        (IPv6, b) => *left = b,
        //
        (Date, Null) => *left = Date,
        (Date, Bool) => *left = Str,
        (Date, Int) => *left = Str,
        (Date, Float) => *left = Str,
        (Date, IPv4) => *left = Str,
        (Date, IPv6) => *left = Str,
        (Date, b) => *left = b,
        //
        (Str, Null) => *left = Str,
        (Str, Bool) => *left = Str,
        (Str, Int) => *left = Str,
        (Str, Float) => *left = Str,
        (Str, IPv4) => *left = Str,
        (Str, IPv6) => *left = Str,
        (Str, Date) => *left = Str,
        (Str, b) => *left = b,
        //
        (Object(_), Null) => (),
        // _ => (),
        _ => {
            unimplemented!()
        }
    }
    return;
}
//
pub fn merge_consume(left: Types, right: Types) -> Types {
    use Types::*;
    match (left, right) {
        (Null, Null) => Null,
        (Bool, Bool) => Bool,
        (Int, Int) => Int,
        (Float, Float) => Float,
        (IPv4, IPv4) => IPv4,
        (IPv6, IPv6) => IPv6,
        (Date, Date) => Date,
        (Str, Str) => Str,
        (Object(mut b), Object(mut a)) => {
            let mut new = BTreeMap::new();
            let mut keys = a.keys().cloned().collect::<Vec<_>>();
            let mut b_keys = b.keys().cloned().collect::<Vec<_>>();
            keys.append(&mut b_keys);
            keys.dedup();
            for key in keys {
                // if let Some(mut a_value) = a.remove(&key) {
                match (a.remove(&key), b.remove(&key)) {
                    (Some(a_value), Some(b_value)) => {
                        new.insert(key, merge_consume(a_value, b_value));
                    }
                    (None, Some(value)) | (Some(value), None) => {
                        new.insert(key, value);
                    }
                    _ => (),
                }
                // }
            }
            Object(new)
        }
        (List(ref mut b), List(mut a)) => {
            let mut new = BTreeMap::new();
            let mut keys = a.keys().cloned().collect::<Vec<_>>();
            let mut b_keys = b.keys().cloned().collect::<Vec<_>>();
            keys.append(&mut b_keys);
            keys.dedup();
            for key in keys {
                // if let Some(mut a_value) = a.remove(&key) {
                match (a.remove(&key), b.remove(&key)) {
                    (Some(a_value), Some(b_value)) => {
                        new.insert(key, merge_consume(a_value, b_value));
                    }
                    (None, Some(value)) | (Some(value), None) => {
                        new.insert(key, value);
                    }
                    _ => (),
                }
                // }
            }
            List(new)
        }
        (List(ref mut a), Object(ref mut b)) | (Object(ref mut b), List(ref mut a)) => {
            // Each list entry merged into Object using i as key.
            let mut new = BTreeMap::new();
            let keys: Vec<usize> = a.keys().cloned().collect();
            for key in keys {
                if let Some(a_value) = a.remove(&key) {
                    match b.remove(&key.to_string()) {
                        Some(b_value) => {
                            new.insert(key.to_string(), merge_consume(a_value, b_value));
                        }
                        None => {
                            new.insert(key.to_string(), a_value);
                        }
                    }
                }
            }
            Object(new)
        }
        (Null, b) => b,
        //
        (Bool, Null) => Bool,
        (Bool, Int) => Int,
        (Bool, Float) => Float,
        (Bool, Str) => Str,
        (Bool, IPv4) => Str,
        (Bool, IPv6) => Str,
        (Bool, Date) => Str,
        (Bool, b) => b,
        //
        (Int, Null) => Int,
        (Int, Bool) => Int,
        (Int, Float) => Float,
        (Int, Str) => Str,
        (Int, IPv4) => Str,
        (Int, IPv6) => Str,
        (Int, Date) => Str,
        (Int, b) => b,
        //
        (Float, Null) => Float,
        (Float, Bool) => Float,
        (Float, Int) => Float,
        (Float, Str) => Str,
        (Float, IPv4) => Str,
        (Float, IPv6) => Float,
        (Float, Date) => Float,
        (Float, b) => b,
        //
        (IPv4, Null) => IPv4,
        (IPv4, Bool) => Str,
        (IPv4, Int) => Str,
        (IPv4, Float) => Str,
        (IPv4, IPv6) => Str,
        (IPv4, b) => b,
        //
        (IPv6, Null) => IPv6,
        (IPv6, Bool) => Str,
        (IPv6, Int) => Str,
        (IPv6, Float) => Str,
        (IPv6, IPv4) => Str,
        (IPv6, b) => b,
        //
        (Date, Null) => Date,
        (Date, Bool) => Str,
        (Date, Int) => Str,
        (Date, Float) => Str,
        (Date, IPv4) => Str,
        (Date, IPv6) => Str,
        (Date, b) => b,
        //
        (Str, Null) => Str,
        (Str, Bool) => Str,
        (Str, Int) => Str,
        (Str, Float) => Str,
        (Str, IPv4) => Str,
        (Str, IPv6) => Str,
        (Str, Date) => Str,
        (Str, b) => b,
        //
        (List(ref mut a), b) => {
            let mut new = BTreeMap::new();
            match a.remove(&0) {
                Some(a_value) => {
                    new.insert(0, merge_consume(a_value, b));
                }
                None => {
                    new.insert(0, b);
                }
            };
            List(new)
        }
        // Unable to handle this at this time.
        (Object(_), _) => {
            unimplemented!()
        }
    }
}
//
pub fn cast_null_with_type(t: &Types) -> Result<Value, Box<dyn Error>> {
    match t {
        Types::Null => Ok(Value::Null),
        Types::Bool => Types::null_bool().map(Value::Bool),
        Types::Int => Types::null_int().map(|i| Value::Number(i.into())),
        Types::Float => Number::from_f64(Types::null_float()?)
            .map(Value::Number)
            .ok_or_else(|| format!("unable to cast null to {:?}", t).into()),
        Types::Str => Types::null_str().map(Value::String),
        _ => Err(format!("unable to cast null to {:?}", t).into()),
    }
}
pub fn cast_bool_with_type(b: bool, t: &Types) -> Result<Value, Box<dyn Error>> {
    match t {
        Types::Null => Types::bool_null(&b).map(|_| Value::Null),
        Types::Bool => Ok(Value::Bool(b)),
        Types::Int => Types::bool_int(&b).map(|i| Value::Number(i.into())),
        Types::Float => Number::from_f64(Types::bool_float(&b)?)
            .map(Value::Number)
            .ok_or_else(|| format!("unable to cast {:?} to {:?}", b, t).into()),
        Types::Str => Types::bool_str(&b).map(Value::String),
        _ => Err(format!("unable to cast {:?} to {:?}", b, t).into()),
    }
}
pub fn cast_int_with_type(i: i64, t: &Types) -> Result<Value, Box<dyn Error>> {
    match t {
        Types::Null => Types::int_null(&i).map(|_| Value::Null),
        Types::Bool => Types::int_bool(&i).map(Value::Bool),
        Types::Int => Ok(Value::Number(i.into())),
        Types::Float => Number::from_f64(Types::int_float(&i)?)
            .map(Value::Number)
            .ok_or_else(|| format!("unable to cast {:?} to {:?}", i, t).into()),
        Types::Str => Types::int_str(&i).map(Value::String),
        _ => Err(format!("unable to cast {:?} to {:?}", i, t).into()),
    }
}
pub fn cast_float_with_type(f: f64, t: &Types) -> Result<Value, Box<dyn Error>> {
    match t {
        Types::Null => Types::float_null(&f).map(|_| Value::Null),
        Types::Bool => Types::float_bool(&f).map(Value::Bool),
        Types::Int => Types::float_int(&f).map(|i| Value::Number(i.into())),
        Types::Float => Number::from_f64(f)
            .map(Value::Number)
            .ok_or_else(|| format!("unable to cast {:?} to {:?}", f, t).into()),
        Types::Str => Types::float_str(&f).map(Value::String),
        _ => Err(format!("unable to cast {:?} to {:?}", f, t).into()),
    }
}
pub fn cast_str_with_type(s: String, t: &Types) -> Result<Value, Box<dyn Error>> {
    match t {
        Types::Null => Types::str_null(&s).map(|_| Value::Null),
        Types::Bool => Types::str_bool(&s).map(Value::Bool),
        Types::Int => Types::str_int(&s).map(|i| Value::Number(i.into())),
        Types::Float => Number::from_f64(Types::str_float(&s)?)
            .map(Value::Number)
            .ok_or_else(|| format!("unable to cast {:?} to {:?}", s, t).into()),
        Types::Str => Ok(Value::String(s)),
        Types::IPv4 => Types::str_ipv4(&s).map(|i| Value::String(i.to_string())),
        Types::IPv6 => Types::str_ipv6(&s).map(|i| Value::String(i.to_string())),
        Types::Date => Types::str_date(&s).map(|d| Value::String(d.to_rfc3339())),
        _ => Err(format!("unable to cast {:?} to {:?}", s, t).into()),
    }
}

pub fn cast_value(t: &Types, v: Value) -> Result<Value, Box<dyn Error>> {
    match (v, t) {
        (Value::Null, Types::Object(_)) => Ok(Value::Null),
        (Value::Null, Types::List(_)) => Ok(Value::Null),
        (Value::Array(mut map), Types::List(type_map)) => {
            let mut casted = Vec::default();
            for (key, value) in map.drain(..).enumerate() {
                match type_map.get(&key) {
                    Some(t) => casted.push(cast_value(t, value)?),
                    // This means we've not accounted for a type in every instance of the array this map has been created for.
                    None => {
                        return Err(
                            format!("no \"{}\" key for type map to cast value to", key).into()
                        )
                    }
                }
            }
            Ok(Value::Array(casted))
        }
        (Value::Object(mut map), Types::Object(type_map)) => {
            let mut casted = serde_json::Map::default();
            let keys = map.keys().cloned().collect::<Vec<_>>();
            for key in keys {
                if let Some(value) = map.remove(&key) {
                    match type_map.get(&key) {
                        Some(t) => casted.insert(key, cast_value(t, value)?),
                        None => {
                            return Err(
                                format!("no \"{}\" key for type map to cast value to", key).into()
                            )
                        }
                    };
                }
            }
            Ok(Value::Object(casted))
        }
        //
        (Value::Null, _) => cast_null_with_type(t),
        (Value::Bool(b), _) => cast_bool_with_type(b, t),
        (Value::Number(n), _) => match n.as_i64() {
            Some(i) => cast_int_with_type(i, t),
            None => match n.as_f64() {
                Some(f) => cast_float_with_type(f, t),
                // Technically unreachable, if the json value exists as a number it cannot fail to resolve to i64 and f64. Potentially if value is in u64 bounds this will be hit.
                None => unreachable!(),
            },
        },
        (Value::String(s), _) => cast_str_with_type(s, t),
        //
        // At this time what is not covered is the Array(_) and Object(_) cases where the right is a primative. These in practice should not occur so they'll remain as errors
        _ => {
            unreachable!()
            // Err(format!("unable to cast {:?} to {:?}", v, self).into())
        }
    }
}

#[cfg(test)]
mod tests;
