use std::collections::BTreeMap;

use chrono::{DateTime, Duration, Utc};

use crate::{
    error::RuntimeError,
    ir::{
        BehaviorTimeProfile, ClockSkew, EventTimeConfig, RangeClockSkew, TimeDelay,
        TimeGranularity, TimeProfile,
    },
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
    event_time: Option<RuntimeEventTimeConfig>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RuntimeEventTimeConfig {
    default: RuntimeBehaviorTimeProfile,
    event_types: BTreeMap<String, RuntimeBehaviorTimeProfile>,
    states: BTreeMap<String, BTreeMap<String, RuntimeBehaviorTimeProfile>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RuntimeBehaviorTimeProfile {
    base_delay: RuntimeTimeDelay,
    behavior_jitter: RuntimeTimeDelay,
    error_jitter: RuntimeTimeDelay,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RuntimeTimeDelay {
    min: Duration,
    max: Duration,
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

        let event_time = match &profile.event_time {
            Some(event_time) => Some(runtime_event_time_config(&profile.id, event_time)?),
            None => None,
        };

        Ok(Self {
            profile_id: profile.id.clone(),
            global_start,
            granularity: profile.granularity,
            monotonic_per_instance: profile.monotonic_per_instance,
            system_clock_skew,
            event_time,
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
            self.event_time.clone(),
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
    event_time: Option<RuntimeEventTimeConfig>,
}

impl InstanceClock {
    fn new(
        start: DateTime<Utc>,
        tick: Duration,
        monotonic: bool,
        event_time: Option<RuntimeEventTimeConfig>,
    ) -> Self {
        Self {
            current: start,
            tick,
            monotonic,
            last_emitted: None,
            event_time,
        }
    }

    pub fn fixed_tick(start: DateTime<Utc>, tick: Duration, monotonic: bool) -> Self {
        Self::new(start, tick, monotonic, None)
    }

    /// Return the timestamp for the next event in this instance.
    ///
    /// With MVP defaults (`monotonic=true`), every call advances by exactly one
    /// profile granularity tick after emitting the current timestamp.
    pub fn next_timestamp(&mut self) -> DateTime<Utc> {
        self.next_timestamp_for_event_type(None, None)
    }

    pub fn next_timestamp_for_event_type(
        &mut self,
        event_type: Option<&str>,
        rng: Option<&mut RuntimeRng>,
    ) -> DateTime<Utc> {
        self.next_timestamp_for_behavior(None, None, event_type, rng)
    }

    pub fn next_timestamp_for_behavior(
        &mut self,
        machine_id: Option<&str>,
        state_id: Option<&str>,
        event_type: Option<&str>,
        rng: Option<&mut RuntimeRng>,
    ) -> DateTime<Utc> {
        let candidate = match self.last_emitted {
            None => self.current,
            Some(last) => {
                let delay = self.sample_delay(machine_id, state_id, event_type, rng);
                last + delay
            }
        };

        let timestamp = if self.monotonic {
            match self.last_emitted {
                Some(last) if candidate <= last => last + self.tick,
                _ => candidate,
            }
        } else {
            candidate
        };

        self.last_emitted = Some(timestamp);
        if self.monotonic {
            self.current = timestamp + self.tick;
        } else {
            self.current = timestamp;
        }

        timestamp
    }

    fn sample_delay(
        &self,
        machine_id: Option<&str>,
        state_id: Option<&str>,
        event_type: Option<&str>,
        rng: Option<&mut RuntimeRng>,
    ) -> Duration {
        let Some(event_time) = &self.event_time else {
            return self.tick;
        };
        let Some(rng) = rng else {
            return self.tick;
        };

        let state_profile = machine_id
            .and_then(|machine_id| event_time.states.get(machine_id))
            .and_then(|states| state_id.and_then(|state_id| states.get(state_id)));
        let event_type_profile =
            event_type.and_then(|event_type| event_time.event_types.get(event_type));
        let profile = state_profile
            .or(event_type_profile)
            .unwrap_or(&event_time.default);

        sample_time_delay(profile.base_delay, rng)
            + sample_time_delay(profile.behavior_jitter, rng)
            + sample_time_delay(profile.error_jitter, rng)
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

fn runtime_event_time_config(
    profile_id: &str,
    config: &EventTimeConfig,
) -> Result<RuntimeEventTimeConfig, RuntimeError> {
    let mut event_types = BTreeMap::new();
    for (event_type, profile) in &config.event_types {
        event_types.insert(
            event_type.clone(),
            runtime_behavior_time_profile(
                profile_id,
                format!("event_time.event_types.{event_type}"),
                profile,
            )?,
        );
    }

    let mut states = BTreeMap::new();
    for (machine_id, machine_states) in &config.states {
        let mut runtime_states = BTreeMap::new();
        for (state_id, profile) in machine_states {
            runtime_states.insert(
                state_id.clone(),
                runtime_behavior_time_profile(
                    profile_id,
                    format!("event_time.states.{machine_id}.{state_id}"),
                    profile,
                )?,
            );
        }
        states.insert(machine_id.clone(), runtime_states);
    }

    Ok(RuntimeEventTimeConfig {
        default: runtime_behavior_time_profile(profile_id, "event_time.default", &config.default)?,
        event_types,
        states,
    })
}

fn runtime_behavior_time_profile(
    profile_id: &str,
    location: impl AsRef<str>,
    profile: &BehaviorTimeProfile,
) -> Result<RuntimeBehaviorTimeProfile, RuntimeError> {
    let location = location.as_ref();
    Ok(RuntimeBehaviorTimeProfile {
        base_delay: runtime_time_delay(
            profile_id,
            format!("{location}.base_delay_ms"),
            &profile.base_delay_ms,
        )?,
        behavior_jitter: runtime_time_delay(
            profile_id,
            format!("{location}.behavior_jitter_ms"),
            &profile.behavior_jitter_ms,
        )?,
        error_jitter: runtime_time_delay(
            profile_id,
            format!("{location}.error_jitter_ms"),
            &profile.error_jitter_ms,
        )?,
    })
}

fn runtime_time_delay(
    profile_id: &str,
    location: impl AsRef<str>,
    delay: &TimeDelay,
) -> Result<RuntimeTimeDelay, RuntimeError> {
    let location = location.as_ref();
    let (min, max) = match delay {
        TimeDelay::Fixed(fixed) => (fixed.milliseconds, fixed.milliseconds),
        TimeDelay::Range(range) => {
            if range.min_milliseconds > range.max_milliseconds {
                return Err(RuntimeError::invalid_time_profile(
                    profile_id,
                    location,
                    "min_milliseconds must be <= max_milliseconds",
                ));
            }
            (range.min_milliseconds, range.max_milliseconds)
        }
    };

    if !min.is_finite() || !max.is_finite() {
        return Err(RuntimeError::invalid_time_profile(
            profile_id,
            location,
            "milliseconds must be finite",
        ));
    }

    Ok(RuntimeTimeDelay {
        min: milliseconds_to_duration_at(profile_id, location, min)?,
        max: milliseconds_to_duration_at(profile_id, location, max)?,
    })
}

fn sample_time_delay(delay: RuntimeTimeDelay, rng: &mut RuntimeRng) -> Duration {
    if delay.min == delay.max {
        return delay.min;
    }

    let min = delay.min.num_nanoseconds().unwrap_or(i64::MIN) as f64;
    let max = delay.max.num_nanoseconds().unwrap_or(i64::MAX) as f64;
    Duration::nanoseconds((min + ((max - min) * rng.next_f64())).round() as i64)
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

    milliseconds_to_duration_at(
        profile_id,
        format!("system_clock_skew.{system_id}"),
        milliseconds,
    )
}

fn milliseconds_to_duration_at(
    profile_id: &str,
    location: impl AsRef<str>,
    milliseconds: f64,
) -> Result<Duration, RuntimeError> {
    let location = location.as_ref();
    if !milliseconds.is_finite() {
        return Err(RuntimeError::invalid_time_profile(
            profile_id,
            location,
            "milliseconds must be finite",
        ));
    }

    let nanoseconds = (milliseconds * 1_000_000.0).round();
    if nanoseconds < i64::MIN as f64 || nanoseconds > i64::MAX as f64 {
        return Err(RuntimeError::invalid_time_profile(
            profile_id,
            location,
            "milliseconds value is outside supported duration range",
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
            event_time: None,
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
