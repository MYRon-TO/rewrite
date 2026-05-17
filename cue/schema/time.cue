package rewrite

// Time profile captures basic time constraints only: global start, granularity,
// per-instance monotonicity and system-level clock skew.

#TimeProfile: close({
	id:    #Identifier
	start: string

	timezone:               *"UTC" | string
	granularity:            *"millisecond" | "second" | "millisecond" | "microsecond" | "nanosecond"
	monotonic_per_instance: *true | bool

	system_clock_skew: *close({}) | {
		[SystemID=#Identifier]: #ClockSkew
	}

	description?: string
})

#ClockSkew: #FixedClockSkew | #RangeClockSkew

#FixedClockSkew: close({
	milliseconds: *0 | number
})

#RangeClockSkew: close({
	min_milliseconds: number
	max_milliseconds: number & >=min_milliseconds
})
