use chrono::DateTime;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::error::Error;
use std::net::{Ipv4Addr, Ipv6Addr};
//
#[derive(Debug, PartialEq, Eq, PartialOrd, Clone, Serialize, Deserialize)]
pub enum Types {
    Null,
    Bool,
    Int,
    Float,
    IPv4,
    IPv6,
    Date,
    Str,
    List(BTreeMap<usize, Types>),
    Object(BTreeMap<String, Types>),
}

impl Default for Types {
    fn default() -> Self {
        Self::Object(BTreeMap::new())
    }
}

impl Types {
    pub fn get_type(v: &Value) -> Self {
        match v {
            Value::Null => Self::Null,
            Value::Number(n) => match n.as_i64() {
                Some(_) => Self::Int,
                None => match n.as_f64() {
                    Some(_) => Self::Float,
                    // Technically unreachable, if the json value exists as a number it cannot fail to resolve to i64 and f64. Potentially if value is in u64 bounds this will be hit.
                    None => unreachable!(),
                },
            },
            Value::Bool(_) => Self::Bool,
            Value::String(v) => Self::test_str(v),
            Value::Array(ref source_map) => {
                let mut dest_map = BTreeMap::default();
                for (key, value) in source_map.iter().enumerate() {
                    dest_map.insert(key, Self::get_type(value));
                }
                Self::List(dest_map)
            }
            Value::Object(ref source_map) => {
                let mut dest_map = BTreeMap::default();
                for (key, value) in source_map {
                    dest_map.insert(key.clone(), Self::get_type(value));
                }
                Self::Object(dest_map)
            }
        }
    }
    //
    // ----------------------------------------------------
    //
    pub fn null_bool() -> Result<bool, Box<dyn Error>> {
        Ok(false)
    }
    pub fn null_int() -> Result<i64, Box<dyn Error>> {
        Ok(0)
    }
    pub fn null_float() -> Result<f64, Box<dyn Error>> {
        Ok(0.0)
    }
    pub fn null_str() -> Result<String, Box<dyn Error>> {
        Ok("null".to_string())
    }
    //
    // ----------------------------------------------------
    //
    pub fn bool_null(_b: &bool) -> Result<(), Box<dyn Error>> {
        // This might need to be changed but default behaviour is to allow casting from anything to Null as in practice this shouldn't happen but could be useful for redacting data.
        Ok(())
    }
    pub fn bool_int(b: &bool) -> Result<i64, Box<dyn Error>> {
        Ok(match b {
            true => 1,
            false => 0,
        })
    }
    pub fn bool_float(b: &bool) -> Result<f64, Box<dyn Error>> {
        Ok(match b {
            true => 1.0,
            false => 0.0,
        })
    }
    pub fn bool_str(b: &bool) -> Result<String, Box<dyn Error>> {
        Ok(b.to_string())
    }
    //
    // ----------------------------------------------------
    //
    pub fn int_null(_i: &i64) -> Result<(), Box<dyn Error>> {
        // This might need to be changed but default behaviour is to allow casting from anything to Null as in practice this shouldn't happen but could be useful for redacting data.
        Ok(())
    }
    pub fn int_bool(i: &i64) -> Result<bool, Box<dyn Error>> {
        match i {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(format!("unable to convert {:?} to bool", i).into()),
        }
    }
    pub fn int_float(i: &i64) -> Result<f64, Box<dyn Error>> {
        // have to step down from i64 (due to serde_json::Value Number impl)
        if *i > (i32::MAX as i64) {
            Ok(i32::MAX as f64) //f64::MAX)
        } else if *i < (i32::MIN as i64) {
            Ok(i32::MIN as f64)
        } else {
            i32::try_from(*i as i64)
                .map(|i| i as f64)
                .map_err(|e| format!("unable to convert {:?} to float, {:?}", i, e).into())
        }
        // Ok(f64::from(*i))
    }
    pub fn int_str(i: &i64) -> Result<String, Box<dyn Error>> {
        Ok(i.to_string())
    }
    //
    // ----------------------------------------------------
    //
    pub fn float_null(_f: &f64) -> Result<(), Box<dyn Error>> {
        // This might need to be changed but default behaviour is to allow casting from anything to Null as in practice this shouldn't happen but could be useful for redacting data.
        Ok(())
    }
    pub fn float_bool(f: &f64) -> Result<bool, Box<dyn Error>> {
        if *f == 0.0 {
            Ok(false)
        } else if *f == 1.0 {
            Ok(true)
        } else {
            Err(format!("unable to convert {:?} to bool", f).into())
        }
    }
    pub fn float_int(f: &f64) -> Result<i64, Box<dyn Error>> {
        Ok(f.round() as i64)
    }
    pub fn float_str(f: &f64) -> Result<String, Box<dyn Error>> {
        Ok(f.to_string())
    }
    //
    // ----------------------------------------------------
    //
    pub fn str_null(s: &str) -> Result<(), Box<dyn Error>> {
        if s.trim().to_ascii_lowercase().as_str() == "null" || s == "0" {
            Ok(())
        } else {
            Err(format!("unable to cast {:?} to null", s).into())
        }
    }
    pub fn str_bool(s: &str) -> Result<bool, Box<dyn Error>> {
        match s.trim().to_ascii_lowercase().as_str() {
            "true" => Ok(true),
            "false" => Ok(false),
            _ => match Self::str_int(s) {
                Ok(0) => Ok(false),
                Ok(1) => Ok(true),
                _ => Err(format!("unable to convert {:?} to bool", s).into()),
            },
        }
    }
    pub fn str_int(s: &str) -> Result<i64, Box<dyn Error>> {
        use std::str::FromStr;
        match i64::from_str(s) {
            Ok(i) => Ok(i),
            _ => match s.trim().to_ascii_lowercase().strip_prefix("0x") {
                Some(hex) => i64::from_str_radix(hex, 16)
                    .map_err(|_| format!("unable to convert {:?} to int", s).into()),
                None => match bool::from_str(s.trim().to_ascii_lowercase().as_str()) {
                    Ok(b) => Ok(i64::from(b)),
                    Err(_) => Err(format!("unable to convert {:?} to int", s).into()),
                },
            },
        }
    }
    pub fn str_float(s: &str) -> Result<f64, Box<dyn Error>> {
        use std::str::FromStr;
        match f64::from_str(s) {
            Ok(f) => Ok(f),
            Err(_) => Err(format!("unable to convert {:?} to float", s).into()),
        }
    }
    pub fn str_ipv4(s: &str) -> Result<Ipv4Addr, Box<dyn Error>> {
        use std::str::FromStr;
        Ipv4Addr::from_str(s).map_err(|_| format!("unable to convert {:?} to ipv4", s).into())
    }
    pub fn str_ipv6(s: &str) -> Result<Ipv6Addr, Box<dyn Error>> {
        use std::str::FromStr;
        Ipv6Addr::from_str(s).map_err(|_| format!("unable to convert {:?} to ipv6", s).into())
    }
    pub fn str_date(s: &str) -> Result<DateTime<chrono::Utc>, Box<dyn Error>> {
        match DateTime::parse_from_rfc3339(s) {
            Ok(dt) => Ok(dt.with_timezone(&chrono::Utc)),
            Err(_) => Err(format!("unable to convert {:?} to timestamp", s).into()),
        }
    }
    pub fn test_str(s: &str) -> Self {
        if Self::str_null(s).is_ok() {
            Self::Null
        } else if Self::str_bool(s).is_ok() {
            Self::Bool
        } else if Self::str_int(s).is_ok() {
            Self::Int
        } else if Self::str_float(s).is_ok() {
            Self::Float
        } else if Self::str_ipv4(s).is_ok() {
            Self::IPv4
        } else if Self::str_ipv6(s).is_ok() {
            Self::IPv6
        } else if Self::str_date(s).is_ok() {
            Self::Date
        } else {
            Self::Str
        }
    }
}
