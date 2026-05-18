# TODO：发布前补齐的静态校验

## 结论

当前阶段只实现 MVP 的必要一致性校验，不按恶意篡改 IR 的场景做完整防御。以下项目先不阻塞阶段二，发布前再补齐。

## 发布前待补

1. **IR 版本白名单**
   - 校验 `ir_version == "0.1"`。
   - 未知版本应拒绝，避免后续 IR 合约变化被旧 runtime 误读。

2. **数值范围语义校验**
   - `emit.count` 固定值与范围值。
   - `{min,max}` 中 `max >= min`。
   - `instance_group.count >= 1`。
   - `clock_skew` 范围中 `max_milliseconds >= min_milliseconds`。

3. **Scenario 非空校验**
   - `scenario.systems` 至少一个。
   - `scenario.instance_groups` 至少一个。

4. **攻击标签语义校验**
   - `machine.attack.enabled == false` 时禁止 `kind = attack` 状态。
   - `kind = benign` 状态不应携带 `attack_label`。
   - 攻击标签字段的最小完整性校验，如 `type/technique/tactic` 的组合规则。

5. **更完整的时间字段格式校验**
   - `time_profile.start` 建议校验为 RFC3339。
   - `timezone` 是否属于可识别时区，可在时间模块落地后统一处理。

## 已在阶段二收敛/补齐

- Drift 扫描范围已收敛：只检查结构性 IR 字段，不扫描 `params`、`template.fields`、`const` 等业务数据载荷。
- 已增加 map key 与内部 `id` 的一致性校验。
- 已增加 `scenario.instance_groups -> machine -> system` 必须落在 `scenario.systems` 内的校验。
- 已增加 `time_profile.system_clock_skew` 的 system key 存在性校验。
