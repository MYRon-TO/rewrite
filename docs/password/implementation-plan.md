# 密码与密钥生成器执行计划

## 1. 已批准范围与稳定决策

本文件把 `docs/password/design.md` 与 `docs/password/plan.md` 转换为可逐步执行、逐步验证的计划。执行日志不写入本文。

用户已确认：

- `data/human_password.txt` 作为内置语料 `human-passwords-v1` 随二进制编译打包；CUE/IR 仍显式使用 `source: {builtin: "human-passwords-v1"}`。
- `base64` 依赖统一采用 `0.23.1`。
- `data/human_password.txt` 超过设计中的推荐条目数不阻塞实现。
- 保持现有 context、template、machine、攻击标签和输出结构不变。
- 子智能体仅在 foreground 中串行运行，每一步一个 fresh worker；worker 不得调用子智能体或提交代码。

## 2. 合约冻结

### 2.1 IR 版本

- Rust 与 CUE 只接受 `ir_version: "0.2"`；不读取、不导出 IR `0.1`。
- 所有现有 CUE 示例和生成的 IR fixture 统一迁移到 `0.2`。
- 项目版本与 IR 主次版本绑定，本阶段将 `Cargo.toml` 的 package version 更新为 `0.2.0`；允许 `Cargo.lock` 随 Cargo 自动同步本项目版本。

### 2.2 `corpus_choice`

参数：

- `source`：必填且只能是 `{builtin: string}` 或 `{path: string}`。
- 唯一内置 ID：`human-passwords-v1`，内容由 `include_str!(.../data/human_password.txt)` 打包。
- `min_length`：默认 `1`，范围 `0..=4096`，按 Unicode 字符数。
- `max_length`：默认 `128`，范围 `0..=4096`，且不得小于 `min_length`。
- `sampling`：默认 `uniform`，可选 `uniform`、`shuffle_cycle`。
- `deduplicate`：默认 `true`。
- `keep_empty`：默认 `false`。
- 未知参数拒绝。

行处理：只移除 LF/CRLF；不 trim；`#` 无注释含义；按配置处理空行、去重和长度过滤；过滤后为空时报错。

外部资源限制：文件最大 16 MiB、最多 1,000,000 行、单行最多 4096 个 Unicode 字符；必须是 UTF-8。相对路径以 `--asset-dir` 为根，否则以 IR 文件所在目录为根。绝对路径直接使用。缓存按规范化数据源保存已读取的原始行；抽样状态按完整有效配置隔离。

### 2.3 `machine_secret`

所有 profile 拒绝未知参数，长度统一限制为 `1..=4096`（PEM 行宽 `1..=256`）。随机值只用于仿真。

- `hex`：必填 `byte_length`，输出小写 Hex。
- `base64`：必填 `byte_length`，标准 Base64。
- `base64url`：必填 `byte_length`，无填充 Base64URL。
- `uuid`：无参数，UUID v4 外形。
- `api_key`：`prefix` 默认 `sim_sk_`，`body_length` 默认 32，`encoding` 默认 `base64url`；encoding 可为 `base64url`、`hex`、`alphanumeric`。
- `bearer_token`：`prefix` 默认 `sim_bearer_`，`body_length` 默认 40，`encoding` 同上；返回 token 本体，不自动附加 HTTP `Bearer `。
- `jwt_shape`：`issuer` 默认 `simulator`，`subject` 默认 `simulated-user`，`signature_length` 默认 32；header/payload 可 Base64URL 解码，signature 仅保证外形。
- `pem_shape`：`label` 默认 `SIMULATED PRIVATE KEY`，`byte_length` 默认 64，`line_width` 默认 64；输出匹配的 BEGIN/END 边界和标准 Base64 主体。
- `cloud_access_key`：`prefix` 默认 `SIMAKIA`，`body_length` 默认 16，主体为大写字母和数字。
- `cloud_secret_pair`：`id_prefix` 默认 `SIMAKIA`，`id_body_length` 默认 16，`secret_length` 默认 40；返回 `{access_key_id, secret_access_key}`。
- `connection_string`：`scheme` 默认 `postgresql`，`host` 默认 `db.invalid`，`port` 默认 5432，`database` 默认 `simdb`，`username` 默认 `sim_user`，`password_length` 默认 32；输出 `scheme://username:password@host:port/database`，固定字段不得为空，port 为 `1..=65535`。
- `private_token`：`prefix` 默认 `sim_tok_`，`body_length` 默认 32，`encoding` 同 API Key。

生成器参数与 context 使用点参数按现有规则合并，使用点参数优先。Rust 对合并后的参数执行最终校验。

## 3. 分步执行

### 步骤 1：CUE/IR 合约与 Rust 静态验证

文件范围：

- `cue/schema/generator.cue`
- `cue/schema/root.cue`
- `cue/examples/*.cue`（将现有示例迁移至 IR 0.2，并保留合法新生成器最小示例）
- `cue/examples/invalid/*password*.cue`（新增/调整）
- `scripts/validate-cue.sh`
- `scripts/validate-invalid-cue.sh`
- `scripts/export-ir.sh`
- `src/validate/generator.rs`（新增）
- `src/validate/mod.rs`
- `src/cli.rs`（仅接入版本/生成器静态验证）
- `Cargo.toml`（把 package version 更新为 `0.2.0`）及 Cargo 自动同步的 `Cargo.lock`
- 必要的 Rust 单元测试与由现有导出流程更新的 generated JSON

任务：项目全面迁移到仅支持 IR `0.2`，实现两个新生成器的 CUE 结构约束、Rust 合并参数后的静态校验，并确保导出的 JSON 使用闭合 `#IR` schema 验证。此步不实现生成逻辑和资产读取。

验证：

- `scripts/validate-cue.sh`
- `scripts/validate-invalid-cue.sh`
- `cargo test validate::generator`
- `cargo test cli::tests`

完成标准：未知 profile、冲突/缺失 source、非法静态长度与任何非 `0.2` IR 被拒绝；全部现有示例迁移至 `0.2`；导出 JSON 经过闭合 `#IR` 验证；项目版本为 `0.2.0`。

### 步骤 2：`machine_secret`

文件范围：

- `src/generator/machine_secret.rs`（新增）
- `src/generator/mod.rs`
- `Cargo.lock`
- 必要的单元测试

任务：实现全部 profile、确定性随机生成、参数校验与对象输出；不得读取或设置攻击标签。

验证：

- `cargo test generator::machine_secret`
- `cargo test generator::tests`
- `cargo check`

完成标准：固定 seed 可复现；编码长度、UUID 位、JWT、PEM、对象字段和非法参数测试通过。

### 步骤 3：`corpus_choice`、缓存与资产根

文件范围：

- `src/generator/corpus_choice.rs`（新增）
- `src/generator/corpus_cache.rs`（新增）
- `src/generator/mod.rs`
- `src/runtime/scenario.rs`
- `src/cli.rs`
- 必要的单元测试

任务：打包内置语料；实现外部文件、限制、过滤、缓存、uniform、shuffle_cycle；为 validate/generate 增加 `--asset-dir`，默认 IR 目录，并把资产根传入静态验证和运行时生成器。

验证：

- `cargo test generator::corpus`
- `cargo test cli::tests`
- `cargo test runtime::scenario`
- `cargo check`

完成标准：内置/外部语料、路径解析、UTF-8/CRLF/空行/空格/#/去重/过滤错误、确定性和 shuffle 一轮不重复均有测试。

### 步骤 4：运行时与端到端示例

文件范围：

- `cue/examples/credential_generators.cue`（新增）
- `tests/integration/credential_generators_e2e.rs`（新增）
- `tests/integration/main.rs`
- `tests/fixtures/credential_generators.json`（如现有测试结构确需）
- `tests/fixtures/golden/credential_generators_seed_42.jsonl`（新增）
- `generated/examples/credential_generators.json`（由现有导出流程生成）
- 必要的集成测试辅助文件

任务：覆盖两个新生成器、弱密码 choice、所有主要 machine profile、四层 context 生命周期、对象子字段模板引用、固定 seed golden 和旧输出兼容。

验证：

- `scripts/validate-cue.sh cue/examples/credential_generators.cue`
- `scripts/export-ir.sh cue/examples/credential_generators.cue`
- `cargo test --test integration credential_generators`
- `cargo test --test e2e`

完成标准：CUE→IR→Rust 验证→日志生成链路通过；相同 seed golden 一致；旧测试不回归。

### 步骤 5：语料说明与用户文档

文件范围：

- `data/NOTICE.md`（新增，记录用户提供语料、仓库内固定版本和处理方式；不得虚构上游许可证）
- `README.md`
- `docs/definition-layer/02-core-objects.md`
- `docs/definition-layer/04-scope-template-reuse.md`
- `docs/definition-layer/05-relations-boundary-drift.md`
- `docs/cue-authoring-guide.md`
- `docs/usage.md`
- `docs/ir-contract.md`（若版本说明集中于此）

任务：说明配置、仿真边界、路径解析、缓存/复现、IR 兼容、语料来源状态以及攻击标签复用规则。若 `data/human_password.txt` 的真实上游来源/许可证未知，明确记录“由项目维护者提供，待补充来源信息”，不冒充许可证已明确。

验证：

- `grep`/人工检查文档示例与实现参数一致
- `cargo test`

完成标准：文档不承诺密码学安全，不声称生成器自动打攻击标签，不虚构语料来源。

### 步骤 6：最终项目验证与审查

不新增功能。运行：

- `cargo fmt --check`
- `make check`
- 固定 seed 生成示例并核对 golden
- `git diff --check`
- 审查完整 diff，确认无无关修改、无 staged 文件、IR 仅支持 `0.2`。

完成标准：全部命令通过，fresh reviewer 无阻塞问题；若 reviewer 提出明确缺陷，只允许一次同步骤返工，仍失败则中止并报告用户。

## 4. 提交规则

- 每一步经 worker 自测和父智能体轻量复核后立即提交。
- worker 不提交；父智能体提交。
- 每一步最多一次返工。
- 最终验证后如只产生必要的格式、golden 或文档修正，单独提交最终收尾；否则不制造空提交。
