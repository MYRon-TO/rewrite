# 日志生成器 MVP 实施计划书

## 目标

在 Rust 技术栈下实现一个最小可用日志生成器：使用 CUE 作为定义层与静态校验工具，但不引入 Rust CUE 库；通过构建前调用 `cue export` 生成规范化 JSON/YAML IR，再由 Rust 程序解析 IR，执行状态机、多实例、模板渲染、上下文变量生成、时间推进，并输出带攻击标签的日志文件。

## 0. 关键结论

1. **MVP 不是只做定义编译，而是要生成可用日志并包含攻击标签。**
2. **CUE 调用方式采用“构建前生成 IR”。** Rust 运行时只读取已生成的 JSON/YAML IR，不直接调用 CUE。
3. **IR 主格式建议用 JSON，YAML 可作为可选输入。** 原因是 `cue export` 到 JSON 更直接，Rust 解析稳定，CI 对比快；如果后续需要人工可读文件，再增加 YAML 导出。
4. **定义层边界严格按现有文档收敛。** 不做 `entity`、跨系统因果、跨 machine 攻击链、事件 DAG、`attack_ratio`。第一版别把自行车焊成火箭，炸了还怪 Rust。

## 1. 范围定义

### 1.1 MVP 必须支持

- CUE schema 与示例定义。
- 构建前 CUE 校验与 IR 导出。
- Rust CLI 读取 IR。
- `system`、`generator`、`context`、`template`、`machine`、`instance_group`、`scenario`、`time_profile` 的核心字段。
- 单系统单实例。
- 单系统多实例。
- 多系统并行生成，但不建跨系统因果。
- `global / system / instance / event` 四层上下文。
- 状态机按 `transition.weight` 随机采样。
- `emit.count` 支持固定值和 `{min,max}`。
- 模板字段支持常量、上下文引用、简单字符串插值。
- 攻击状态、攻击标签继承与覆盖。
- 输出日志文件，至少支持 JSON Lines。
- 基础测试、样例场景和 drift 检查清单。

### 1.2 MVP 明确不支持

- Rust 直接解析 CUE AST。
- 运行时调用 CUE。
- 任意脚本执行。
- 跨实例依赖。
- 跨系统依赖。
- 跨 machine 攻击链。
- 顶层 `attack_chain`。
- `instance_group.attack_ratio`。
- 复杂因果 DAG。
- 复杂表达式语言。
- 插件式外部 generator。

## 2. 总体架构

### 2.1 数据流

```text
CUE definitions
  -> cue vet / cue export
  -> generated IR: target/definition/scenario.json
  -> Rust CLI reads IR
  -> static validation
  -> runtime context initialization
  -> instance_group expansion
  -> state machine execution
  -> event generation
  -> template rendering
  -> attack label injection
  -> JSONL output
```

### 2.2 建议目录结构

```text
cue/
  schema/
    root.cue
    system.cue
    generator.cue
    context.cue
    template.cue
    machine.cue
    scenario.cue
    time.cue
  examples/
    ssh_bruteforce.cue
    multi_system.cue
scripts/
  export-ir.sh
  validate-cue.sh
generated/
  examples/
    ssh_bruteforce.json
src/
  cli.rs
  ir/
  validate/
  runtime/
  generator/
  context/
  template/
  machine/
  label/
  output/
tests/
  fixtures/
  integration/
docs/
  implementation-plan.md
```

### 2.3 Rust 模块边界

- `ir`：只定义反序列化结构，不包含运行逻辑。
- `validate`：做 CUE 不方便表达或需要跨引用检查的静态校验。
- `generator`：实现有限内置生成器。
- `context`：管理四层作用域变量解析与缓存。
- `template`：渲染事件字段，不判断攻击。
- `machine`：执行状态转移和事件发射，不渲染字段。
- `label`：根据 `state.kind`、`state.attack_label`、`machine.attack` 生成攻击标签。
- `runtime`：组装 scenario、system、instance_group 的执行。
- `output`：负责 JSONL 写出。
- `cli`：解析命令行参数并调用 runtime。

## 3. 阶段划分原则

1. **阶段按依赖顺序推进。** 后一阶段只依赖前一阶段交付的稳定接口。
2. **同一阶段功能点必须互相独立。** 每个功能点有明确文件归属、输入、输出、验收标准。
3. **先锁 IR，再写 runtime。** 否则 Rust 结构和 CUE schema 会互相拉扯，最后变成配置界的意大利面。
4. **先生成 JSONL，再考虑其他输出。** 输出格式泛化放到后续版本。

## 4. 阶段 0：项目基建与开发约束

### 目标

建立可并行开发的工程骨架、命令入口和质量门禁。

### 并行功能点

#### 0.1 Cargo 工程与依赖基线

- 负责人可独立修改：`Cargo.toml`、`src/main.rs`、`src/lib.rs`。
- 建议依赖：`serde`、`serde_json`、可选 `serde_yaml`、`clap`、`rand`、`chrono` 或 `time`、`thiserror`、`anyhow`、`indexmap`。
- 输出：可运行的 CLI 空壳。
- 验收：`cargo check`、`cargo test` 通过。

#### 0.2 CUE 工具脚本

- 负责人可独立修改：`scripts/validate-cue.sh`、`scripts/export-ir.sh`。
- 输入：`cue/examples/*.cue`。
- 输出：`generated/examples/*.json`。
- 验收：本机无 Rust 参与即可完成 CUE 校验和导出。

#### 0.3 CI/本地任务约定

- 负责人可独立修改：`Makefile` 或 `justfile`、CI 配置。
- 命令建议：`check`、`test`、`cue-validate`、`cue-export`、`run-example`。
- 验收：一条命令能完成校验、导出、测试。

#### 0.4 错误模型约定

- 负责人可独立修改：`src/error.rs`。
- 输出：统一错误分类，如 `IrParseError`、`ValidationError`、`RuntimeError`、`OutputError`。
- 验收：各模块只依赖错误类型，不互相依赖实现。

### 阶段 0 完成标准

- 工程可以构建。
- CUE CLI 已纳入本地流程。
- 后续模块有稳定目录和错误边界。

## 5. 阶段 1：定义层 Schema 与 IR 合约

### 目标

把文档中的定义层对象落成 CUE schema 和规范化 IR，作为后续 Rust 模块的唯一输入契约。

### 并行功能点

#### 1.1 核心对象 CUE schema

- 文件归属：`cue/schema/*.cue`。
- 范围：`system`、`machine`、`template`、`generator`、`context`、`scenario`、`instance_group`、`time_profile`。
- 不做：执行逻辑、随机逻辑、跨系统因果。
- 验收：非法字段会被 CUE 拦截。

#### 1.2 攻击分支 Schema

- 文件归属：`cue/schema/machine.cue`。
- 范围：`machine.attack`、`state.kind`、`state.attack_label`、`transition.weight`、`emit.count`。
- 验收：
  - `transition.weight > 0`。
  - `emit.count.min <= emit.count.max`。
  - `attack.enabled == false` 时禁止攻击状态。

#### 1.3 上下文与模板绑定 Schema

- 文件归属：`cue/schema/context.cue`、`cue/schema/template.cue`。
- 范围：常量、上下文引用、简单插值表达。
- 验收：模板不能声明共享策略，不能包含脚本字段。

#### 1.4 IR 文档

- 文件归属：`docs/ir-contract.md`。
- 内容：JSON 字段、默认值展开规则、引用格式、版本字段。
- 验收：Rust 开发者只看 IR 文档即可实现反序列化。

#### 1.5 示例场景

- 文件归属：`cue/examples/ssh_bruteforce.cue`。
- 内容：Linux SSH 登录正常路径与爆破攻击路径。
- 验收：能导出规范化 JSON IR。

### 阶段 1 完成标准

- `cue export` 能输出一个完整 scenario IR。
- IR 中默认值已规范化。
- 引用关系表达稳定。

## 6. 阶段 2：Rust IR 读取与静态校验

### 目标

Rust 能稳定读取构建前生成的 IR，并在执行前发现引用错误和边界漂移。

### 并行功能点

#### 2.1 IR 数据结构

- 文件归属：`src/ir/`。
- 输入：`generated/examples/*.json`。
- 输出：Rust 结构体。
- 验收：示例 IR 可成功反序列化。

#### 2.2 引用完整性校验

- 文件归属：`src/validate/references.rs`。
- 校验：
  - `scenario.systems` 引用存在。
  - `instance_group.machine` 引用存在。
  - `machine.system` 引用存在。
  - `state.emit.template` 引用存在。
  - `template` 引用的 context 变量存在。
- 验收：缺失引用返回定位明确的错误。

#### 2.3 Machine 结构校验

- 文件归属：`src/validate/machine.rs`。
- 校验：
  - `start` 状态存在。
  - `transition.to` 状态存在。
  - 非终止状态不能无转移且无明确终止。
  - 可选：检测明显不可达状态并警告。
- 验收：坏状态机不能进入 runtime。

#### 2.4 定义边界漂移校验

- 文件归属：`src/validate/drift.rs`。
- 校验：禁止 IR 中出现 MVP 不支持对象，如 `entity`、`attack_chain`、`attack_ratio`、`stage`、事件 DAG 字段。
- 验收：越界字段或扩展对象被拒绝。

#### 2.5 CLI 输入入口

- 文件归属：`src/cli.rs`。
- 命令建议：`rewrite validate --ir generated/examples/ssh_bruteforce.json`。
- 验收：不执行生成，仅验证 IR。

### 阶段 2 完成标准

- Rust 可读取 IR。
- 引用错误、状态机错误、边界漂移能提前失败。
- `validate` 命令可用。

## 7. 阶段 3：运行时基础能力

### 目标

实现状态机执行前所需的基础运行组件，但暂不串成完整日志生成流程。

### 并行功能点

#### 3.1 随机与种子管理

- 文件归属：`src/runtime/rng.rs`。
- 输入：CLI seed 或默认 seed。
- 输出：可复现实验的随机源。
- 验收：相同 seed 产生相同采样序列。

#### 3.2 内置 Generator

- 文件归属：`src/generator/`。
- MVP 支持：
  - 固定值。
  - 从列表随机选择。
  - 数字范围。
  - IPv4 简单生成。
  - 用户名简单生成。
  - UUID/session id。
  - 时间戳由 time 模块提供。
- 验收：每个 generator 可独立单测。

#### 3.3 Context Store

- 文件归属：`src/context/`。
- 范围：`global`、`system`、`instance`、`event` 四层作用域。
- 规则：
  - `global/system/instance` 可缓存复用。
  - `event` 每条事件重新生成。
- 验收：同一 instance 内变量稳定，不同 instance 的 instance 变量独立。

#### 3.4 时间模型

- 文件归属：`src/runtime/time.rs`。
- MVP 支持：全局起始时间、单实例单调递增、系统级 clock skew。
- 验收：同一 instance 事件时间不倒退。

#### 3.5 值表达与插值解析

- 文件归属：`src/template/value_expr.rs`。
- 支持：常量、`ctx.global.*`、`ctx.system.*`、`ctx.instance.*`、`ctx.event.*`、简单字符串插值。
- 不支持：任意函数调用、复杂条件、循环。
- 验收：表达式解析模块不依赖 machine。

### 阶段 3 完成标准

- 基础能力均可单测。
- 还不要求生成完整日志，避免过早耦合。

## 8. 阶段 4：状态机执行与事件组装

### 目标

把 instance_group 展开为多个独立 instance，执行状态机并产生未渲染或已半结构化事件。

### 并行功能点

#### 4.1 Instance Group 展开

- 文件归属：`src/runtime/instance_group.rs`。
- 输入：scenario 中启用的 instance_group。
- 输出：instance 运行计划列表。
- 验收：多实例共享 global/system，隔离 instance context。

#### 4.2 Transition Sampler

- 文件归属：`src/machine/sampler.rs`。
- 输入：当前状态 transitions。
- 输出：下一个状态 ID。
- 规则：按同一状态下 `weight` 归一化采样。
- 验收：采样分布在统计测试中接近权重。

#### 4.3 Emit Count Sampler

- 文件归属：`src/machine/emit.rs`。
- 输入：`count` 固定值或 `{min,max}`。
- 输出：本状态同类事件数量。
- 验收：范围合法，边界可测。

#### 4.4 Machine Runner

- 文件归属：`src/machine/runner.rs`。
- 输入：machine、instance context、rng。
- 输出：事件草稿列表，包括 system、machine、state、template、时间上下文。
- 验收：遇到 terminal 停止；设置最大步数保护，防死循环。

#### 4.5 多系统执行协调

- 文件归属：`src/runtime/scenario.rs`。
- 范围：多 system 并行参与，同享 global context。
- 不做：跨系统因果、跨系统触发。
- 验收：两个系统可以各自生成事件，互不引用。

### 阶段 4 完成标准

- 给定 IR 可以生成事件草稿。
- 多实例互相独立。
- 攻击路径由 transition weight 进入，而不是 attack_ratio。

## 9. 阶段 5：模板渲染、攻击标签与输出

### 目标

将事件草稿渲染为最终日志记录，并输出 JSON Lines。

### 并行功能点

#### 5.1 Template Renderer

- 文件归属：`src/template/renderer.rs`。
- 输入：template、context store、event draft。
- 输出：业务字段 map。
- 验收：模板只渲染字段，不判断攻击。

#### 5.2 Attack Label Resolver

- 文件归属：`src/label/attack.rs`。
- 规则：
  - `state.kind == attack` 时 `label.is_attack = true`。
  - `state.kind == benign` 时 `label.is_attack = false`。
  - 标签优先级：`state.attack_label` 覆盖 `machine.attack`。
  - 输出 `attack_state`、`attack_machine`。
- 验收：同一 template 在 benign 和 attack 状态下标签不同。

#### 5.3 Event Assembler

- 文件归属：`src/runtime/event.rs`。
- 输入：渲染字段、标签、元数据、时间戳。
- 输出：统一事件结构。
- 验收：字段命名稳定，保留 scenario/system/instance/machine/state/template 元信息。

#### 5.4 JSONL Output Writer

- 文件归属：`src/output/jsonl.rs`。
- 输入：事件流。
- 输出：`.jsonl` 文件。
- 验收：每行合法 JSON；大文件采用流式写出。

#### 5.5 Generate CLI

- 文件归属：`src/cli.rs`。
- 命令建议：
  - `rewrite validate --ir generated/examples/ssh_bruteforce.json`
  - `rewrite generate --ir generated/examples/ssh_bruteforce.json --out out/events.jsonl --seed 42`
- 验收：示例命令可生成日志。

### 阶段 5 完成标准

- MVP 主流程打通。
- 输出日志包含攻击标签。
- 相同 seed 输出可复现。

## 10. 阶段 6：样例、测试与交付硬化

### 目标

补齐可演示样例、自动化测试和文档，确保 MVP 可被继续扩展。

### 并行功能点

#### 6.1 SSH 爆破端到端样例

- 文件归属：`cue/examples/ssh_bruteforce.cue`、`tests/fixtures/`。
- 验收：生成日志中同时存在 benign 与 attack 事件。

#### 6.2 多实例样例

- 文件归属：`cue/examples/multi_instance_login.cue`。
- 验收：多个 instance 共享 system/global，不共享 instance 变量。

#### 6.3 多系统并行样例

- 文件归属：`cue/examples/multi_system.cue`。
- 验收：多个 system 共享 global，但没有跨系统依赖。

#### 6.4 Snapshot / Golden Tests

- 文件归属：`tests/integration/`。
- 输入：固定 seed 与固定 IR。
- 验收：输出与 golden 文件一致。

#### 6.5 失败用例测试

- 文件归属：`tests/fixtures/invalid/`。
- 覆盖：缺模板、缺状态、攻击字段越界、非法 weight、非法 count、禁止对象。
- 验收：错误消息可定位。

#### 6.6 用户文档

- 文件归属：`README.md`、`docs/usage.md`。
- 内容：安装 CUE CLI、导出 IR、运行 Rust CLI、查看输出。
- 验收：新环境按文档能跑通示例。

### 阶段 6 完成标准

- 项目具备演示、测试和基本文档。
- 未来扩展不会破坏 MVP 边界。

## 11. 接口冻结点

### 11.1 Phase 1 后冻结

- IR 顶层结构。
- 对象 ID 与引用格式。
- `machine.states` 表达方式。
- `template.fields` 表达方式。
- `context` 作用域表达方式。

### 11.2 Phase 3 后冻结

- Generator 内置类型名称。
- Context 变量解析接口。
- 时间 API。

### 11.3 Phase 5 后冻结

- JSONL 输出事件字段。
- 攻击标签字段。
- CLI 参数名称。

## 12. 建议事件输出结构

MVP 每条 JSONL 事件建议包含：

```text
{
  "timestamp": "...",
  "scenario": "...",
  "system": "...",
  "instance": "...",
  "machine": "...",
  "state": "...",
  "template": "...",
  "fields": { ... },
  "label": {
    "is_attack": true/false,
    "attack_type": "...",
    "technique": "...",
    "tactic": "...",
    "attack_state": "...",
    "attack_machine": "..."
  }
}
```

其中 `label` 始终存在，攻击字段在非攻击事件中可为空或省略，但 `is_attack` 必须存在。

## 13. 验收标准

### 13.1 功能验收

- 能通过 CUE CLI 从示例定义导出 IR。
- Rust CLI 能读取 IR 并完成静态校验。
- 能生成 JSONL 日志。
- 支持多实例。
- 支持多系统并行。
- 支持攻击状态和攻击标签。
- 支持 `emit.count` 生成多条事件。
- 支持按 `transition.weight` 进入攻击路径。

### 13.2 边界验收

- 不出现 Rust CUE 库。
- Rust 运行时不调用 CUE。
- 不出现 `entity`、`attack_chain`、`attack_ratio`。
- 模板不判断攻击。
- `scenario` 不编排攻击链。
- 多系统之间没有因果依赖。

### 13.3 质量验收

- `cargo test` 通过。
- CUE 校验脚本通过。
- 固定 seed 的端到端输出稳定。
- 错误信息能定位到对象 ID 和字段。

## 14. 风险与控制

### 风险 1：IR 频繁变化导致 Rust 返工

- 控制：Phase 1 先冻结 IR 合约；Phase 2 之后只允许兼容变更。

### 风险 2：模板表达式膨胀成脚本语言

- 控制：MVP 只支持常量、上下文引用、简单字符串插值。

### 风险 3：攻击比例被误解为全局配额

- 控制：文档和校验中明确 `transition.weight` 只是局部分支概率，不提供 `attack_ratio`。

### 风险 4：多系统被偷偷做成因果编排

- 控制：validate drift 检查禁止跨系统依赖字段。

### 风险 5：Generator 过早插件化

- 控制：MVP 只做内置 generator，外部插件留到后续版本。

## 15. 推荐开发顺序

1. Phase 0：工程骨架、CUE 脚本、质量命令。
2. Phase 1：CUE schema、IR 合约、SSH 爆破示例。
3. Phase 2：Rust IR 读取与静态校验。
4. Phase 3：随机、generator、context、time、表达式解析。
5. Phase 4：instance_group 展开、状态机执行、事件草稿。
6. Phase 5：模板渲染、攻击标签、JSONL 输出、generate CLI。
7. Phase 6：样例、集成测试、文档与边界硬化。

## 16. 最小首个可演示版本

首个 demo 只需要一个 `ssh_bruteforce.cue`：

- 一个 `linux_auth` system。
- 一个 `ssh_login_flow` machine。
- 正常路径：登录成功 + 普通活动。
- 攻击路径：多次登录失败 + 攻击成功 + 可疑 sudo。
- 一个 `instance_group`，实例数可配置。
- 输出 JSONL 中能看到：
  - 正常事件 `is_attack=false`。
  - 爆破事件 `is_attack=true`、`technique=T1110`。
  - 攻击成功事件可覆盖为 `technique=T1078`。

这就是 MVP 的最小闭环。
