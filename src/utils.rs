use byte_unit::{Byte, UnitType};
use std::time::Duration;
use humantime::format_duration;

pub fn byte2str(num: u64, binary: bool) -> String {
    let byte = Byte::from_u64(num);
    let rslt = byte.get_appropriate_unit(if binary {UnitType::Binary} else {UnitType::Decimal});
    format!("{:.2} {}", rslt.get_value(), rslt.get_unit())
}

pub fn str2byte(s: &str) -> u64 {
    Byte::parse_str(s, false).unwrap().as_u64()
}

pub fn s2time(seconds: u64) -> String {
    format_duration(Duration::from_secs(seconds)).to_string()
}
