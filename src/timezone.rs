use windows_timezones::WindowsTimezone;

fn windows_to_iana(key: &str) -> Result<String, ()> {
    key.parse::<WindowsTimezone>()
        .map(|zone| zone.tzdb_id().to_owned())
        .map_err(|_| ())
}

#[cfg(windows)]
pub fn detected_timezone() -> Result<String, ()> {
    use std::mem::MaybeUninit;
    use windows_sys::Win32::System::Time::{
        DYNAMIC_TIME_ZONE_INFORMATION, GetDynamicTimeZoneInformation,
    };

    let mut zone = MaybeUninit::<DYNAMIC_TIME_ZONE_INFORMATION>::zeroed();
    let result = unsafe { GetDynamicTimeZoneInformation(zone.as_mut_ptr()) };
    if result == u32::MAX {
        return Err(());
    }
    let zone = unsafe { zone.assume_init() };
    let length = zone
        .TimeZoneKeyName
        .iter()
        .position(|value| *value == 0)
        .ok_or(())?;
    let key = String::from_utf16(&zone.TimeZoneKeyName[..length]).map_err(|_| ())?;
    windows_to_iana(&key)
}

#[cfg(not(windows))]
pub fn detected_timezone() -> Result<String, ()> {
    Err(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_windows_zones_without_localized_display_names() {
        for (windows, iana) in [
            ("E. South America Standard Time", "America/Sao_Paulo"),
            ("UTC", "Etc/UTC"),
            ("Pacific Standard Time", "America/Los_Angeles"),
            ("W. Europe Standard Time", "Europe/Berlin"),
            ("Tokyo Standard Time", "Asia/Tokyo"),
            ("India Standard Time", "Asia/Calcutta"),
        ] {
            assert_eq!(windows_to_iana(windows).as_deref(), Ok(iana));
        }
    }

    #[test]
    fn unknown_zone_is_an_error_instead_of_a_guessed_timezone() {
        assert!(windows_to_iana("Unknown timezone").is_err());
        assert!(windows_to_iana("").is_err());
    }
}
