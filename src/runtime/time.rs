use std::collections::BTreeMap;

use chrono::{DateTime, Duration, Utc};

use crate::{
    error::RuntimeError,
    ir::{ClockSkew, RangeClockSkew, TimeGranularity, TimeProfile},
    runtime::rng::RuntimeRng,
};

/// Runtime time model derived from one IR `time_profile`.
///
/// MVP semantics are intentionally small:
/// - one global RFC3339 start time;
/// - one optional system-level clock skew applied to every instance in that system;
/// - one per-instance clock that never moves backwards when `monotonic_per_instance` is true.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimeModel {
    profile_id: String,
    global_start: DateTime<Utc>,
    granularity: TimeGranularity,
    monotonic_per_instance: bool,
    system_clock_skew: BTreeMap<String, Duration>,
}

impl TimeModel {
    pub fn from_profile(profile: &TimeProfile, rng: &mut RuntimeRng) -> Result<Self, RuntimeError> {
        let global_start = DateTime::parse_from_rfc3339(&profile.start)
            .map_err(|source| {
                RuntimeError::invalid_time_profile(
                    &profile.id,
                    "start",
                    format!("expected RFC3339 timestamp: {source}"),
                )
            })?
            .with_timezone(&Utc);

        let mut system_clock_skew = BTreeMap::new();
        for (system_id, skew) in &profile.system_clock_skew {
            let skew = sample_clock_skew(&profile.id, system_id, skew, rng)?;
            system_clock_skew.insert(system_id.clone(), skew);
        }

        Ok(Self {
            profile_id: profile.id.clone(),
            global_start,
            granularity: profile.granularity,
            monotonic_per_instance: profile.monotonic_per_instance,
            system_clock_skew,
        })
    }

    pub fn profile_id(&self) -> &str {
        &self.profile_id
    }

    pub fn global_start(&self) -> DateTime<Utc> {
        self.global_start
    }

    pub fn granularity(&self) -> TimeGranularity {
        self.granularity
    }

    pub fn tick(&self) -> Duration {
        granularity_duration(self.granularity)
    }

    pub fn skew_for_system(&self, system_id: &str) -> Duration {
        self.system_clock_skew
            .get(system_id)
            .copied()
            .unwrap_or_else(Duration::zero)
    }

    pub fn start_for_system(&self, system_id: &str) -> DateTime<Utc> {
        self.global_start + self.skew_for_system(system_id)
    }

    pub fn instance_clock(&self, system_id: &str) -> InstanceClock {
        InstanceClock::new(
            self.start_for_system(system_id),
            self.tick(),
            self.monotonic_per_instance,
        )
    }
}

/// Per-instance timestamp cursor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstanceClock {
    current: DateTime<Utc>,
    tick: Duration,
    monotonic: bool,
    last_emitted: Option<DateTime<Utc>>,
}

impl InstanceClock {
    pub fn new(start: DateTime<Utc>, tick: Duration, monotonic: bool) -> Self {
        Self {
            current: start,
            tick,
            monotonic,
            last_emitted: None,
        }
    }

    /// Return the timestamp for the next event in this instance.
    ///
    /// With MVP defaults (`monotonic=true`), every call advances by exactly one
    /// profile granularity tick after emitting the current timestamp.
    pub fn next_timestamp(&mut self) -> DateTime<Utc> {
        let timestamp = if self.monotonic {
            match self.last_emitted {
                Some(last) if self.current <= last => last + self.tick,
                _ => self.current,
            }
        } else {
            self.current
        };

        self.last_emitted = Some(timestamp);
        if self.monotonic {
            self.current = timestamp + self.tick;
        }

        timestamp
    }

    pub fn peek(&self) -> DateTime<Utc> {
        self.current
    }
}

pub fn granularity_duration(granularity: TimeGranularity) -> Duration {
    match granularity {
        TimeGranularity::Second => Duration::seconds(1),
        TimeGranularity::Millisecond => Duration::milliseconds(1),
        TimeGranularity::Microsecond => Duration::microseconds(1),
        TimeGranularity::Nanosecond => Duration::nanoseconds(1),
    }
}

fn sample_clock_skew(
    profile_id: &str,
    system_id: &str,
    skew: &ClockSkew,
    rng: &mut RuntimeRng,
) -> Result<Duration, RuntimeError> {
    let milliseconds = match skew {
        ClockSkew::Fixed(fixed) => fixed.milliseconds,
        ClockSkew::Range(RangeClockSkew {
            min_milliseconds,
            max_milliseconds,
        }) => {
            if min_milliseconds > max_milliseconds {
                return Err(RuntimeError::invalid_time_profile(
                    profile_id,
                    format!("system_clock_skew.{system_id}"),
                    "min_milliseconds must be <= max_milliseconds",
                ));
            }
            min_milliseconds + ((*max_milliseconds - *min_milliseconds) * rng.next_f64())
        }
    };

    milliseconds_to_duration(profile_id, system_id, milliseconds)
}

fn milliseconds_to_duration(
    profile_id: &str,
    system_id: &str,
    milliseconds: f64,
) -> Result<Duration, RuntimeError> {
    if !milliseconds.is_finite() {
        return Err(RuntimeError::invalid_time_profile(
            profile_id,
            format!("system_clock_skew.{system_id}"),
            "clock skew milliseconds must be finite",
        ));
    }

    let nanoseconds = (milliseconds * 1_000_000.0).round();
    if nanoseconds < i64::MIN as f64 || nanoseconds > i64::MAX as f64 {
        return Err(RuntimeError::invalid_time_profile(
            profile_id,
            format!("system_clock_skew.{system_id}"),
            "clock skew is outside supported duration range",
        ));
    }

    Ok(Duration::nanoseconds(nanoseconds as i64))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{FixedClockSkew, RangeClockSkew};

    fn profile() -> TimeProfile {
        TimeProfile {
            id: "default_time".to_string(),
            start: "2026-05-17T00:00:00Z".to_string(),
            timezone: "UTC".to_string(),
            granularity: TimeGranularity::Millisecond,
            monotonic_per_instance: true,
            system_clock_skew: BTreeMap::new(),
            description: None,
        }
    }

    #[test]
    fn creates_global_start_and_monotonic_instance_clock() {
        let mut rng = RuntimeRng::new(42);
        let model = TimeModel::from_profile(&profile(), &mut rng).expect("valid profile");
        let mut clock = model.instance_clock("linux_auth");

        let first = clock.next_timestamp();
        let second = clock.next_timestamp();
        let third = clock.next_timestamp();

        assert_eq!(first.to_rfc3339(), "2026-05-17T00:00:00+00:00");
        assert_eq!(second - first, Duration::milliseconds(1));
        assert_eq!(third - second, Duration::milliseconds(1));
        assert!(first <= second && second <= third);
    }

    #[test]
    fn applies_fixed_system_clock_skew() {
        let mut profile = profile();
        profile.system_clock_skew.insert(
            "linux_auth".to_string(),
            ClockSkew::Fixed(FixedClockSkew {
                milliseconds: 1500.0,
            }),
        );

        let mut rng = RuntimeRng::new(42);
        let model = TimeModel::from_profile(&profile, &mut rng).expect("valid profile");

        assert_eq!(
            model.start_for_system("linux_auth") - model.global_start(),
            Duration::milliseconds(1500)
        );
        assert_eq!(model.start_for_system("unknown"), model.global_start());
    }

    #[test]
    fn samples_range_system_clock_skew_inside_bounds() {
        let mut profile = profile();
        profile.system_clock_skew.insert(
            "linux_auth".to_string(),
            ClockSkew::Range(RangeClockSkew {
                min_milliseconds: -50.0,
                max_milliseconds: 50.0,
            }),
        );

        let mut rng = RuntimeRng::new(7);
        let model = TimeModel::from_profile(&profile, &mut rng).expect("valid profile");
        let skew = model.skew_for_system("linux_auth");

        assert!(skew >= Duration::milliseconds(-50));
        assert!(skew <= Duration::milliseconds(50));
    }

    #[test]
    fn rejects_invalid_start_timestamp() {
        let mut profile = profile();
        profile.start = "not-a-date".to_string();

        let mut rng = RuntimeRng::new(42);
        let err = TimeModel::from_profile(&profile, &mut rng).expect_err("invalid profile");

        assert!(matches!(err, RuntimeError::InvalidTimeProfile { .. }));
    }

    #[test]
    fn rejects_invalid_skew_range() {
        let mut profile = profile();
        profile.system_clock_skew.insert(
            "linux_auth".to_string(),
            ClockSkew::Range(RangeClockSkew {
                min_milliseconds: 10.0,
                max_milliseconds: -10.0,
            }),
        );

        let mut rng = RuntimeRng::new(42);
        let err = TimeModel::from_profile(&profile, &mut rng).expect_err("invalid skew");

        assert!(matches!(err, RuntimeError::InvalidTimeProfile { .. }));
    }
}
