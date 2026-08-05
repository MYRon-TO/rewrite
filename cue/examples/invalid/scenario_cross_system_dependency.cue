package rewrite

ir_version: "0.2"
systems: {a: {}, b: {}}
machines: {m: {system: "a", start: "s", states: {s: {terminal: true}}}}
instance_groups: {g: {machine: "m", count: 1}}
time_profiles: {tp: {start: "2026-01-01T00:00:00Z"}}
scenario: {
  id: "bad_cross_system_dependency"
  systems: ["a", "b"]
  instance_groups: ["g"]
  time_profile: "tp"
  depends_on: [{from: "a", to: "b"}]
}
