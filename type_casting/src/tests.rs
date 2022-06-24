use crate::types::Types;
use crate::{cast_value, merge, merge_consume};
use serde_json::{json, Value};
//
fn merge_objects() {}
//
mod serde_json_value {
    use super::*;
    #[test]
    fn object_1_consume() {
        let value_1 = json!({"a": 1, "b": 2.0, "c": 1});
        println!("{value_1}");
        let type_map_1 = Types::get_type(&value_1);
        println!("1: {:?}", type_map_1);
        //
        let value_2 = json!({"a": 1, "b": "2.a", "c": "1.0.2.4"});
        println!("{value_2}");
        let type_map_2 = Types::get_type(&value_2);
        println!("2: {:?}", type_map_2);
        //
        assert!(type_map_1 != type_map_2);
        //
        let merged_1 = merge_consume(type_map_1.clone(), type_map_2.clone());
        println!("1: {:?}", merged_1);
        let merged_2 = merge_consume(type_map_2, type_map_1);
        println!("2: {:?}", merged_2);
        //
        assert_eq!(merged_1, merged_2);
    }
    //
    #[test]
    fn object_1_borrow() {
        let value_1 = json!({"a": 1, "b": 2.0, "c": 1});
        println!("{value_1}");
        let type_map_1 = Types::get_type(&value_1);
        println!("1: {:?}", type_map_1);
        //
        let value_2 = json!({"a": 1, "b": "2.a", "c": "1.0.2.4"});
        println!("{value_2}");
        let type_map_2 = Types::get_type(&value_2);
        println!("2: {:?}", type_map_2);
        //
        assert!(type_map_1 != type_map_2);
        //
        let mut merged_1 = type_map_1.clone();
        merge(&mut merged_1, type_map_2.clone());
        println!("1: {:?}", merged_1);
        //
        let mut merged_2 = type_map_2.clone();
        merge(&mut merged_2, type_map_1.clone());
        println!("2: {:?}", merged_2);
        //
        assert_eq!(merged_1, merged_2);
    }
}
mod map_merges {
    use super::*;
    #[test]
    fn null() {
        assert_eq!(merge_consume(Types::Null, Types::Bool), Types::Bool);
        assert_eq!(merge_consume(Types::Null, Types::Int), Types::Int);
        assert_eq!(merge_consume(Types::Null, Types::Float), Types::Float);
        assert_eq!(merge_consume(Types::Null, Types::Str), Types::Str);
        assert_eq!(merge_consume(Types::Null, Types::IPv4), Types::IPv4);
        assert_eq!(merge_consume(Types::Null, Types::IPv6), Types::IPv6);
        assert_eq!(merge_consume(Types::Null, Types::Date), Types::Date);
    }
}
//
mod null_casts {
    use super::*;
    #[test]
    fn null_bool() {
        let v_ok_1 = Value::Null;
        //
        let type_map = Types::Bool;
        //
        assert_eq!(
            cast_value(&type_map, v_ok_1).map_err(|_| ()),
            Ok(Value::Bool(false))
        );
    }
    #[test]
    fn null_int() {
        let v_ok_1 = Value::Null;
        //
        let type_map = Types::Int;
        //
        assert_eq!(cast_value(&type_map, v_ok_1).map_err(|_| ()), Ok(json!(0)));
    }
    #[test]
    fn null_float() {
        let v_ok_1 = Value::Null;
        //
        let type_map = Types::Float;
        //
        assert_eq!(
            cast_value(&type_map, v_ok_1).map_err(|_| ()),
            Ok(json!(0.0))
        );
    }
    #[test]
    fn null_str() {
        let v_ok_1 = Value::Null;
        //
        let type_map = Types::Str;
        //
        assert_eq!(
            cast_value(&type_map, v_ok_1).map_err(|_| ()),
            Ok(json!("null"))
        );
    }
}
mod bool_casts {
    use super::*;
    #[test]
    fn bool_null() {
        let v_ok_1 = Value::Bool(false);
        let v_err_1 = Value::Bool(true);
        //
        let type_map = Types::Null;
        //
        assert_eq!(
            cast_value(&type_map, v_ok_1).map_err(|_| ()),
            Ok(Value::Null)
        );
        assert_eq!(
            cast_value(&type_map, v_err_1).map_err(|_| ()),
            Ok(Value::Null)
        );
    }
    #[test]
    fn bool_int() {
        let v_ok_1 = Value::Bool(false);
        let v_ok_2 = Value::Bool(true);
        //
        let type_map = Types::Int;
        //
        assert_eq!(cast_value(&type_map, v_ok_1).map_err(|_| ()), Ok(json!(0)));
        assert_eq!(cast_value(&type_map, v_ok_2).map_err(|_| ()), Ok(json!(1)));
    }
    #[test]
    fn bool_float() {
        let v_ok_1 = Value::Bool(false);
        let v_ok_2 = Value::Bool(true);
        //
        let type_map = Types::Float;
        //
        assert_eq!(
            cast_value(&type_map, v_ok_1).map_err(|_| ()),
            Ok(json!(0.0))
        );
        assert_eq!(
            cast_value(&type_map, v_ok_2).map_err(|_| ()),
            Ok(json!(1.0))
        );
    }
    #[test]
    fn bool_str() {
        let v_ok_1 = Value::Bool(false);
        let v_ok_2 = Value::Bool(true);
        //
        let type_map = Types::Str;
        //
        assert_eq!(
            cast_value(&type_map, v_ok_1).map_err(|_| ()),
            Ok(json!("false"))
        );
        assert_eq!(
            cast_value(&type_map, v_ok_2).map_err(|_| ()),
            Ok(json!("true"))
        );
    }
}
mod int_casts {
    use super::*;
    #[test]
    fn int_null() {
        let v_ok_1 = json!(0);
        let v_ok_2 = json!(i64::MIN);
        let v_ok_3 = json!(i64::MAX);
        //
        let type_map = Types::Null;
        //
        assert_eq!(
            cast_value(&type_map, v_ok_1).map_err(|_| ()),
            Ok(Value::Null)
        );
        assert_eq!(
            cast_value(&type_map, v_ok_2).map_err(|_| ()),
            Ok(Value::Null)
        );
        assert_eq!(
            cast_value(&type_map, v_ok_3).map_err(|_| ()),
            Ok(Value::Null)
        );
    }
    #[test]
    fn int_bool() {
        let v_ok_1 = json!(0);
        let v_ok_2 = json!(1);
        let v_err_1 = json!(i64::MIN);
        let v_err_2 = json!(i64::MAX);
        //
        let type_map = Types::Bool;
        //
        assert_eq!(
            cast_value(&type_map, v_ok_1).map_err(|_| ()),
            Ok(Value::Bool(false))
        );
        assert_eq!(
            cast_value(&type_map, v_ok_2).map_err(|_| ()),
            Ok(Value::Bool(true))
        );
        assert_eq!(cast_value(&type_map, v_err_1).map_err(|_| ()), Err(()));
        assert_eq!(cast_value(&type_map, v_err_2).map_err(|_| ()), Err(()));
    }
    //     #[test]
    //     fn int_float() {
    //         let v_ok_1 = json!(0.into());
    //         let v_ok_2 = json!(1.into());
    //         // let v_ok_3 = json!(-1.into());
    //         let v_ok_4 = json!(i64::MIN);
    //         let v_ok_5 = json!(i64::MAX);
    //         //
    //         let type_map = Types::Float;
    //         //
    //         assert_eq!(
    //             cast_value(&type_map, v_ok_1).map_err(|_| ()),
    //             Ok(json!(0.0))
    //         );
    //         assert_eq!(
    //             cast_value(&type_map, v_ok_2).map_err(|_| ()),
    //             Ok(json!(1.0))
    //         );
    //         // assert_eq!(
    //         //     cast_value(&type_map, v_ok_3).map_err(|_| ()),
    //         //     Ok(json!(-1.0))
    //         // );
    //         assert_eq!(
    //             type_map.cast_value(v_ok_4).map_err(|_| ()),
    //             Ok(json!(-2147483648.0))
    //         );
    //         assert_eq!(
    //             type_map.cast_value(v_ok_5).map_err(|_| ()),
    //             Ok(json!(2147483647.0))
    //         );
    //     }
    //     #[test]
    //     fn int_string() {
    //         let v_ok_1 = Value::Int(0);
    //         let v_ok_2 = Value::Int(1);
    //         let v_ok_3 = Value::Int(-1);
    //         let v_ok_4 = Value::Int(i64::MIN);
    //         let v_ok_5 = Value::Int(i64::MAX);
    //         //
    //         let type_map = ElasticTypesInner::<Value>::Str;
    //         //
    //         assert_eq!(
    //             type_map.cast_value(v_ok_1).map_err(|_| ()),
    //             Ok(Value::Str("0".to_string()))
    //         );
    //         assert_eq!(
    //             type_map.cast_value(v_ok_2).map_err(|_| ()),
    //             Ok(Value::Str("1".to_string()))
    //         );
    //         assert_eq!(
    //             type_map.cast_value(v_ok_3).map_err(|_| ()),
    //             Ok(Value::Str("-1".to_string()))
    //         );
    //         assert_eq!(
    //             type_map.cast_value(v_ok_4).map_err(|_| ()),
    //             Ok(Value::Str("-9223372036854775808".to_string()))
    //         );
    //         assert_eq!(
    //             type_map.cast_value(v_ok_5).map_err(|_| ()),
    //             Ok(Value::Str("9223372036854775807".to_string()))
    //         );
    //     }
    // }
    // mod float_casts {
    //     use super::*;
    //     #[test]
    //     fn int_null() {
    //         let v_ok_1 = Value::Int(0);
    //         let v_ok_2 = Value::Int(i64::MIN);
    //         let v_ok_3 = Value::Int(i64::MAX);
    //         //
    //         let type_map = ElasticTypesInner::<Value>::Null;
    //         //
    //         assert_eq!(type_map.cast_value(v_ok_1).map_err(|_| ()), Ok(Value::Null));
    //         assert_eq!(type_map.cast_value(v_ok_2).map_err(|_| ()), Ok(Value::Null));
    //         assert_eq!(type_map.cast_value(v_ok_3).map_err(|_| ()), Ok(Value::Null));
    //     }
    //     #[test]
    //     fn float_bool() {
    //         let v_ok_1 = Value::Float(0.0);
    //         let v_ok_2 = Value::Float(1.0);
    //         let v_err_1 = Value::Float(f64::MIN);
    //         let v_err_2 = Value::Float(f64::MAX);
    //         //
    //         let type_map = ElasticTypesInner::<Value>::Bool;
    //         //
    //         assert_eq!(
    //             type_map.cast_value(v_ok_1).map_err(|_| ()),
    //             Ok(Value::Bool(false))
    //         );
    //         assert_eq!(
    //             type_map.cast_value(v_ok_2).map_err(|_| ()),
    //             Ok(Value::Bool(true))
    //         );
    //         assert_eq!(type_map.cast_value(v_err_1).map_err(|_| ()), Err(()));
    //         assert_eq!(type_map.cast_value(v_err_2).map_err(|_| ()), Err(()));
    //     }
    //     #[test]
    //     fn float_int() {
    //         let v_ok_1 = Value::Float(0.0);
    //         let v_ok_2 = Value::Float(1.0);
    //         let v_ok_3 = Value::Float(-1.0);
    //         let v_ok_4 = Value::Float(f64::MIN);
    //         let v_ok_5 = Value::Float(f64::MAX);
    //         //
    //         let type_map = ElasticTypesInner::<Value>::Int;
    //         //
    //         assert_eq!(
    //             type_map.cast_value(v_ok_1).map_err(|_| ()),
    //             Ok(Value::Int(0))
    //         );
    //         assert_eq!(
    //             type_map.cast_value(v_ok_2).map_err(|_| ()),
    //             Ok(Value::Int(1))
    //         );
    //         assert_eq!(
    //             type_map.cast_value(v_ok_3).map_err(|_| ()),
    //             Ok(Value::Int(-1))
    //         );
    //         assert_eq!(
    //             type_map.cast_value(v_ok_4).map_err(|_| ()),
    //             Ok(Value::Int(-9223372036854775808))
    //         );
    //         assert_eq!(
    //             type_map.cast_value(v_ok_5).map_err(|_| ()),
    //             Ok(Value::Int(9223372036854775807))
    //         );
    //     }
    //     #[test]
    //     fn float_str() {
    //         let v_ok_1 = Value::Float(0.0);
    //         let v_ok_2 = Value::Float(1.0);
    //         let v_ok_3 = Value::Float(-1.0);
    //         let v_ok_4 = Value::Float(f64::MIN);
    //         let v_ok_5 = Value::Float(f64::MAX);
    //         //
    //         let type_map = ElasticTypesInner::<Value>::Str;
    //         //
    //         assert_eq!(
    //             type_map.cast_value(v_ok_1).map_err(|_| ()),
    //             Ok(Value::Str("0".to_string()))
    //         );
    //         assert_eq!(
    //             type_map.cast_value(v_ok_2).map_err(|_| ()),
    //             Ok(Value::Str("1".to_string()))
    //         );
    //         assert_eq!(
    //             type_map.cast_value(v_ok_3).map_err(|_| ()),
    //             Ok(Value::Str("-1".to_string()))
    //         );
    //         assert_eq!(
    //                 type_map.cast_value(v_ok_4).map_err(|_|()),
    //                 Ok(Value::Str("-179769313486231570000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000".to_string()))
    //             );
    //         assert_eq!(
    //                 type_map.cast_value(v_ok_5).map_err(|_|()),
    //                 Ok(Value::Str("179769313486231570000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000".to_string()))
    //             );
    //     }
    // }
    // //
    // mod str_casts {
    //     use super::*;
    //     #[test]
    //     fn str_null() {
    //         let v_ok_1 = Value::Str("0".to_string());
    //         let v_ok_2 = Value::Str("Null".to_string());
    //         let v_err_1 = Value::Str("nool".to_string());
    //         let v_err_2 = Value::Str("1".to_string());
    //         //
    //         let type_map = ElasticTypesInner::<Value>::Null;
    //         //
    //         assert_eq!(type_map.cast_value(v_ok_1).map_err(|_| ()), Ok(Value::Null));
    //         assert_eq!(type_map.cast_value(v_ok_2).map_err(|_| ()), Ok(Value::Null));
    //         assert_eq!(type_map.cast_value(v_err_1).map_err(|_| ()), Err(()));
    //         assert_eq!(type_map.cast_value(v_err_2).map_err(|_| ()), Err(()));
    //     }
    //     #[test]
    //     fn str_bool() {
    //         let v_ok_1 = Value::Str("0".to_string());
    //         let v_ok_2 = Value::Str("1".to_string());
    //         let v_ok_3 = Value::Str("fAlse".to_string());
    //         let v_ok_4 = Value::Str("tRue".to_string());
    //         let v_err_1 = Value::Str("2".to_string());
    //         let v_err_2 = Value::Str("eslaf".to_string());
    //         //
    //         let type_map = ElasticTypesInner::<Value>::Bool;
    //         //
    //         assert_eq!(
    //             type_map.cast_value(v_ok_1).map_err(|_| ()),
    //             Ok(Value::Bool(false))
    //         );
    //         assert_eq!(
    //             type_map.cast_value(v_ok_2).map_err(|_| ()),
    //             Ok(Value::Bool(true))
    //         );
    //         assert_eq!(
    //             type_map.cast_value(v_ok_3).map_err(|_| ()),
    //             Ok(Value::Bool(false))
    //         );
    //         assert_eq!(
    //             type_map.cast_value(v_ok_4).map_err(|_| ()),
    //             Ok(Value::Bool(true))
    //         );
    //         assert_eq!(type_map.cast_value(v_err_1).map_err(|_| ()), Err(()));
    //         assert_eq!(type_map.cast_value(v_err_2).map_err(|_| ()), Err(()));
    //     }
    //     #[test]
    //     fn str_int() {
    //         let v_ok_1 = Value::Str("0".to_string());
    //         let v_ok_2 = Value::Str("9223372036854775807".to_string());
    //         let v_ok_3 = Value::Str("-9223372036854775808".to_string());
    //         let v_ok_4 = Value::Str("0x1234".to_string());
    //         let v_err_1 = Value::Str("0x".to_string());
    //         let v_err_2 = Value::Str("not_an_int".to_string());
    //         let v_err_3 = Value::Str("2,147,483,647".to_string());
    //         let v_err_4 = Value::Str("9223372036854775808".to_string());
    //         let v_err_5 = Value::Str("-9223372036854775809".to_string());
    //         //
    //         let type_map = ElasticTypesInner::<Value>::Int;
    //         //
    //         assert_eq!(
    //             type_map.cast_value(v_ok_1).map_err(|_| ()),
    //             Ok(Value::Int(0))
    //         );
    //         assert_eq!(
    //             type_map.cast_value(v_ok_2).map_err(|_| ()),
    //             Ok(Value::Int(i64::MAX))
    //         );
    //         assert_eq!(
    //             type_map.cast_value(v_ok_3).map_err(|_| ()),
    //             Ok(Value::Int(i64::MIN))
    //         );
    //         assert_eq!(
    //             type_map.cast_value(v_ok_4).map_err(|_| ()),
    //             Ok(Value::Int(4660))
    //         );
    //         assert_eq!(type_map.cast_value(v_err_1).map_err(|_| ()), Err(()));
    //         assert_eq!(type_map.cast_value(v_err_2).map_err(|_| ()), Err(()));
    //         assert_eq!(type_map.cast_value(v_err_3).map_err(|_| ()), Err(()));
    //         assert_eq!(type_map.cast_value(v_err_4).map_err(|_| ()), Err(()));
    //         assert_eq!(type_map.cast_value(v_err_5).map_err(|_| ()), Err(()));
    //     }
    //     #[test]
    //     fn str_float() {
    //         let v_ok_1 = json!("0".to_string());
    //         let v_ok_2 = json!(f64::MAX.to_string());
    //         let v_ok_3 = json!(f64::MIN.to_string());
    //         let _v_ok_4 = json!("0x1234".to_string());
    //         let v_ok_5 = json!("1.0E+123".to_string());
    //         let v_err_1 = json!("0x".to_string());
    //         let v_err_2 = json!("not_an_int".to_string());
    //         let v_err_3 = json!("2,147,483,647".to_string());
    //         let v_err_4 = json!("2.147.483.647".to_string());
    //         //
    //         let type_map = Types::Float;
    //         //
    //         assert_eq!(
    //             cast_value(&type_map, v_ok_1).map_err(|_| ()),
    //             Ok(json!(0.into()))
    //         );
    //         assert_eq!(
    //             cast_value(&type_map, v_ok_2).map_err(|_| ()),
    //             Ok(json!(f64::MAX))
    //         );
    //         assert_eq!(
    //             cast_value(&type_map, v_ok_3).map_err(|_| ()),
    //             Ok(json!(f64::MIN))
    //         );
    //         // assert_eq!(type_map.cast_value(v_ok_4), Ok(Value::Float(4660.0)));
    //         // assert_eq!(
    //         //     type_map.cast_value(v_ok_5).map_err(|_| ()),
    //         //     Ok(json!(1e123))
    //         // );
    //         assert_eq!(cast_value(&type_map, v_err_1).map_err(|_| ()), Err(()));
    //         assert_eq!(cast_value(&type_map, v_err_2).map_err(|_| ()), Err(()));
    //         assert_eq!(cast_value(&type_map, v_err_3).map_err(|_| ()), Err(()));
    //         assert_eq!(cast_value(&type_map, v_err_4).map_err(|_| ()), Err(()));
    //     }
    #[test]
    fn str_ipv4() {
        let v_ok_1 = json!("0.0.0.0");
        let v_ok_2 = json!("0.0.255.255");
        let v_ok_3 = json!("255.255.255.255");
        let v_err_1 = json!("0x12.12.12.12");
        //
        let type_map = Types::IPv4;
        //
        assert_eq!(
            cast_value(&type_map, v_ok_1).map_err(|_| ()),
            Ok(json!("0.0.0.0"))
        );
        assert_eq!(
            cast_value(&type_map, v_ok_2).map_err(|_| ()),
            Ok(json!("0.0.255.255"))
        );
        assert_eq!(
            cast_value(&type_map, v_ok_3).map_err(|_| ()),
            Ok(json!("255.255.255.255"))
        );
        assert_eq!(cast_value(&type_map, v_err_1).map_err(|_| ()), Err(()));
    }
    #[test]
    fn str_ipv6() {
        let v_ok_1 = json!("684d:1111:222:3333:4444:5555:6:77");
        let v_ok_2 = json!("2001:db8::1");
        let v_ok_3 = json!("0:0:0:0:0:0:0:1");
        let v_ok_4 = json!("::1");
        let v_err_1 = json!("1.0.0.0");
        //
        let type_map = Types::IPv6;
        //
        assert_eq!(
            cast_value(&type_map, v_ok_1).map_err(|_| ()),
            Ok(json!("684d:1111:222:3333:4444:5555:6:77"))
        );
        assert_eq!(
            cast_value(&type_map, v_ok_2).map_err(|_| ()),
            Ok(json!("2001:db8::1"))
        );
        assert_eq!(
            cast_value(&type_map, v_ok_3).map_err(|_| ()),
            Ok(json!("::1"))
        );
        assert_eq!(
            cast_value(&type_map, v_ok_4).map_err(|_| ()),
            Ok(json!("::1"))
        );
        assert_eq!(cast_value(&type_map, v_err_1).map_err(|_| ()), Err(()));
    }
}
