package rewrite

// Time profile captures global start, granularity, per-instance monotonicity,
// system-level clock skew, and optional event_type-based event delays.

#TimeProfile: close({
	id:    #Identifier
	start: string

	timezone:               *"UTC" | string
	granularity:            *"millisecond" | "second" | "millisecond" | "microsecond" | "nanosecond"
	monotonic_per_instance: *true | bool

	system_clock_skew: *close({}) | {
		[SystemID=#Identifier]: #ClockSkew
	}

	event_time?: close({
		default: #BehaviorTimeProfile
		event_types?: *close({}) | {
			[EventType=string]: #BehaviorTimeProfile
		}
	})
	description?: string
})

#BehaviorTimeProfile: close({
	base_delay_ms:     #TimeDelay
	behavior_jitter_ms: #TimeDelay
	error_jitter_ms:    #TimeDelay
})

#TimeDelay: #FixedTimeDelay | #RangeTimeDelay

#FixedTimeDelay: close({
	milliseconds: number
})

#RangeTimeDelay: close({
	min_milliseconds: number
	max_milliseconds: number & >=min_milliseconds
})
#ClockSkew: #FixedClockSkew | #RangeClockSkew

#FixedClockSkew: close({
	milliseconds: *0 | number
})

#RangeClockSkew: close({
	min_milliseconds: number
	max_milliseconds: number & >=min_milliseconds
})
