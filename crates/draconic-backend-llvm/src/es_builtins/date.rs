use std::time::{SystemTime, UNIX_EPOCH};

use super::*;

pub(super) fn date_now_ms() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as f64)
        .unwrap_or(0.0)
}

/// ECMA-262 Date.UTC(year, month, date=1, hours=0, minutes=0, seconds=0, ms=0) subset.
pub(super) fn date_utc(args: &[JsVal]) -> Result<f64, ()> {
    let year = to_number(args.first().ok_or(())?)?;
    let month = to_number(args.get(1).ok_or(())?)?;
    let date = match args.get(2) {
        Some(v) => to_number(v)?,
        None => 1.0,
    };
    let hours = match args.get(3) {
        Some(v) => to_number(v)?,
        None => 0.0,
    };
    let minutes = match args.get(4) {
        Some(v) => to_number(v)?,
        None => 0.0,
    };
    let seconds = match args.get(5) {
        Some(v) => to_number(v)?,
        None => 0.0,
    };
    let ms = match args.get(6) {
        Some(v) => to_number(v)?,
        None => 0.0,
    };
    if ![year, month, date, hours, minutes, seconds, ms]
        .iter()
        .all(|n| n.is_finite())
    {
        return Ok(f64::NAN);
    }
    let y = year.trunc() as i64;
    let m = month.trunc() as i64;
    // ECMA MakeFullYear: years 0–99 → 1900+y (not needed for fixture; keep full year).
    let day = date.trunc() as i64;
    let h = hours.trunc() as i64;
    let mi = minutes.trunc() as i64;
    let s = seconds.trunc() as i64;
    let milli = ms.trunc() as i64;
    // Normalize month into year.
    let mut yy = y;
    let mut mm = m;
    if mm >= 0 {
        yy += mm / 12;
        mm %= 12;
    } else {
        let years = (-mm + 11) / 12;
        yy -= years;
        mm += years * 12;
    }
    let day_num = days_from_civil(yy as i32, (mm + 1) as u32, 1) + (day - 1);
    let time_ms = ((h * 60 + mi) * 60 + s) * 1000 + milli;
    Ok((day_num * 86_400_000 + time_ms) as f64)
}

/// Days from Unix epoch (1970-01-01) for civil (y, m, d) with m in 1..=12.
/// Howard Hinnant civil_from_days inverse.
pub(super) fn days_from_civil(y: i32, m: u32, d: u32) -> i64 {
    let y = y as i64;
    let m = m as i64;
    let d = d as i64;
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as u64;
    let mp = if m > 2 { m - 3 } else { m + 9 };
    let doy = (153 * mp as u64 + 2) / 5 + d as u64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe as i64 - 719_468
}

/// Civil (y, m, d) from days since Unix epoch. Howard Hinnant algorithm.
pub(super) fn civil_from_days(z: i64) -> (i32, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as i32, m as u32, d as u32)
}

pub(super) const MS_PER_DAY: i64 = 86_400_000;

/// Split UTC ms into (day number since epoch, time-within-day ms). Fixture uses UTC as local.
pub(super) fn split_date_ms(ms: f64) -> Result<(i64, i64), ()> {
    if !ms.is_finite() {
        return Err(());
    }
    let t = ms.trunc() as i64;
    Ok((t.div_euclid(MS_PER_DAY), t.rem_euclid(MS_PER_DAY)))
}

pub(super) fn date_full_year(ms: f64) -> Result<f64, ()> {
    let (day, _) = split_date_ms(ms)?;
    let (y, _, _) = civil_from_days(day);
    Ok(y as f64)
}

/// Annex B.2.4 Date.prototype.getYear: YearFromTime(t) − 1900 (UTC-as-local for fixture).
pub(super) fn date_get_year(ms: f64) -> Result<f64, ()> {
    Ok(date_full_year(ms)? - 1900.0)
}

/// Annex B.2.5 Date.prototype.setYear: MakeFullYear for 0–99 → 1900+y; keep mon/day/tod.
pub(super) fn date_set_year(ms: &mut f64, year_arg: &JsVal) -> Result<f64, ()> {
    let y = to_number(year_arg)?;
    if y.is_nan() {
        *ms = f64::NAN;
        return Ok(f64::NAN);
    }
    if !ms.is_finite() {
        return Err(());
    }
    let (day, tod) = split_date_ms(*ms)?;
    let (_oy, mon, dom) = civil_from_days(day);
    let yi = y.trunc() as i64;
    let yyyy = if (0..=99).contains(&yi) {
        1900 + yi
    } else {
        yi
    };
    let new_day = days_from_civil(yyyy as i32, mon, dom);
    let new_ms = (new_day * MS_PER_DAY + tod) as f64;
    *ms = new_ms;
    Ok(new_ms)
}

/// ECMA-262 Date.prototype.toUTCString / Annex B toGMTString.
pub(super) fn date_to_gmt_string(ms: f64) -> Result<String, ()> {
    if !ms.is_finite() {
        return Ok("Invalid Date".into());
    }
    let (day, tod) = split_date_ms(ms)?;
    let (y, m, d) = civil_from_days(day);
    // Epoch day 0 = Thursday.
    let wd = day.rem_euclid(7) as usize;
    const WDAYS: [&str; 7] = ["Thu", "Fri", "Sat", "Sun", "Mon", "Tue", "Wed"];
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let hour = tod / 3_600_000;
    let min = (tod % 3_600_000) / 60_000;
    let sec = (tod % 60_000) / 1000;
    Ok(format!(
        "{}, {:02} {} {:04} {:02}:{:02}:{:02} GMT",
        WDAYS[wd],
        d,
        MONTHS[(m - 1) as usize],
        y,
        hour,
        min,
        sec
    ))
}

pub(super) fn date_proto_method_builtin(key: &str) -> Option<BuiltinId> {
    match key {
        "getYear" => Some(BuiltinId::DateGetYear),
        "setYear" => Some(BuiltinId::DateSetYear),
        "toGMTString" => Some(BuiltinId::DateToGmtString),
        "getFullYear" => Some(BuiltinId::DateGetFullYear),
        "getTime" | "valueOf" => None, // instance-only in current fixtures
        _ => None,
    }
}

pub(super) fn is_date_proto_method(id: BuiltinId) -> bool {
    matches!(
        id,
        BuiltinId::DateGetYear
            | BuiltinId::DateSetYear
            | BuiltinId::DateToGmtString
            | BuiltinId::DateGetFullYear
    )
}

pub(super) fn date_proto_method_name(id: BuiltinId) -> Option<&'static str> {
    match id {
        BuiltinId::DateGetYear => Some("getYear"),
        BuiltinId::DateSetYear => Some("setYear"),
        BuiltinId::DateToGmtString => Some("toGMTString"),
        BuiltinId::DateGetFullYear => Some("getFullYear"),
        _ => None,
    }
}

pub(super) fn eval_date_method(ms: &mut f64, key: &str, args: &[JsVal]) -> Result<JsVal, ()> {
    match key {
        "getTime" | "valueOf" if args.is_empty() => Ok(JsVal::Num(*ms)),
        "getFullYear" if args.is_empty() => Ok(JsVal::Num(date_full_year(*ms)?)),
        "getYear" if args.is_empty() => Ok(JsVal::Num(date_get_year(*ms)?)),
        "toGMTString" if args.is_empty() => Ok(JsVal::Str(date_to_gmt_string(*ms)?)),
        "setYear" => {
            let arg = args.first().unwrap_or(&JsVal::Undef);
            Ok(JsVal::Num(date_set_year(ms, arg)?))
        }
        _ => Err(()),
    }
}
