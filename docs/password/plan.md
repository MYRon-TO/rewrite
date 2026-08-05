# 密码与密钥生成器实现计划

## 1. 实施范围

本计划实现：

- `machine_secret` 内置生成器；
- `corpus_choice` 内置生成器；
- 内置小型密码语料和外部语料覆盖；
- 新生成器的 CUE 与 Rust 参数校验；
- IR `0.2` 及对 `0.1` 的读取兼容；
- 单元、集成和端到端测试；
- 相关定义与使用文档。

弱密码继续使用现有 `choice`。攻击标签继续使用现有 machine/state 机制，不在本计划中修改标签架构。

## 2. 阶段一：冻结生成器合约

### 任务

1. 确定 `machine_secret` profile、参数名、默认值和资源上限。
2. 确定 `corpus_choice` 数据源、过滤和抽样参数。
3. 定义新生成器在 IR `0.2` 中的结构。
4. 定义 `0.1` 和 `0.2` 的兼容及验证规则。
5. 定义外部资源路径解析规则。

### 主要文件

- `cue/schema/generator.cue`
- `cue/schema/root.cue`
- `src/ir/mod.rs`
- `src/validate/generator.rs`
- `src/validate/mod.rs`

### 验收

- CUE 能拒绝未知 profile、冲突数据源和非法长度。
- Rust 能读取现有 `0.1` IR 和新增 `0.2` IR。
- `0.1` IR 使用新生成器时验证失败。

## 3. 阶段二：实现 `machine_secret`

### 任务

1. 新增 `src/generator/machine_secret.rs`。
2. 提取随机字节、字符集、Hex、Base64 和 Base64URL 辅助函数。
3. 实现以下 profile：
   - `hex`
   - `base64`
   - `base64url`
   - `uuid`
   - `api_key`
   - `bearer_token`
   - `jwt_shape`
   - `pem_shape`
   - `cloud_access_key`
   - `cloud_secret_pair`
   - `connection_string`
   - `private_token`
4. 在 `src/generator/mod.rs` 注册和分派新类型。
5. 实现 profile 参数的 Rust 校验。
6. 保持生成器仅返回值，不读取或设置攻击标签。

### 依赖

在 `Cargo.toml` 增加：

```toml
base64 = "0.22"
```

同步更新 `Cargo.lock`。

### 测试

- 相同 seed 输出一致，不同 seed 输出不同。
- Hex、Base64 和 Base64URL 解码长度正确。
- UUID 版本位和 variant 正确。
- JWT 为三段式且 header、payload 可解码。
- PEM 边界、换行和 Base64 主体正确。
- 对象型 profile 字段完整。
- 非法长度、编码和参数组合被拒绝。

## 4. 阶段三：实现 `corpus_choice`

### 任务

1. 新增 `src/generator/corpus_choice.rs`。
2. 新增 `src/generator/corpus_cache.rs`。
3. 支持内置语料 ID 和本地文件路径。
4. 在 CLI 增加 `--asset-dir`，默认使用 IR 文件所在目录。
5. 实现 LF/CRLF、UTF-8、空行、去重和长度过滤规则。
6. 实现 `uniform` 和 `shuffle_cycle`。
7. 设置文件大小、行数和单行长度上限。
8. 缓存已加载的规范化语料源。

### 内置语料

1. 选择许可证明确的数据来源。
2. 生成 1,000 至 10,000 条固定版本子集。
3. 新增：
   - `assets/password/human-passwords-v1.txt`
   - `assets/password/NOTICE.md`
   - 可重复执行的整理脚本和来源记录。
4. 不提交完整泄露库或身份组合数据。

### 测试

- 内置和外部数据源均可使用。
- 相对路径按 `--asset-dir` 或 IR 目录解析。
- 同 seed、同语料结果可复现。
- CRLF、Unicode、空行和前后空格处理正确。
- `#` 不被解释为注释。
- 空文件、非法 UTF-8、文件缺失和过滤后为空时失败。
- `shuffle_cycle` 在一轮内不重复。
- 同一语料只加载一次。

## 5. 阶段四：运行时集成

### 任务

1. 扩展 `BuiltinGenerator`，持有 asset root 和 corpus cache。
2. 将 IR 文件目录或 `--asset-dir` 传入生成器。
3. 保持现有 context 生命周期和缓存语义不变。
4. 确保对象型生成值可继续通过 context 字段路径引用。
5. 确保未使用新生成器时不改变现有输出。

### 主要文件

- `src/generator/mod.rs`
- `src/runtime/scenario.rs`
- `src/context/mod.rs`
- `src/cli.rs`

### 测试

- `global`、`system` 和 `instance` 值按原规则复用。
- `event` 值按事件重新生成。
- 多实例和多系统场景结果可复现。
- `cloud_secret_pair` 子字段可被模板引用。
- 未使用新功能的 golden 输出不变。

## 6. 阶段五：示例与端到端验证

### 新增示例

新增 `cue/examples/credential_generators.cue`，覆盖：

- 内置语料人工密码；
- `choice` 弱密码；
- API Key；
- Bearer Token；
- JWT 外形；
- PEM 外形；
- 云凭据；
- 含凭据的连接字符串。

示例使用现有 context、template 和 machine 结构，不新增字段级标注或输出结构。

### 新增测试

- `tests/integration/credential_generators_e2e.rs`
- `tests/fixtures/credential_generators.json`
- `tests/fixtures/golden/credential_generators_seed_42.jsonl`
- 新生成器对应的非法 CUE fixtures。

### 验收

- `make check` 全部通过。
- 固定 seed 的输出与 golden 文件一致。
- 两个新生成器均通过 CUE 导出、IR 验证和日志生成流程。
- 现有示例和测试保持通过。

## 7. 阶段六：文档更新

### 任务

1. 更新以下文档：
   - `docs/definition-layer/02-core-objects.md`
   - `docs/definition-layer/04-scope-template-reuse.md`
   - `docs/definition-layer/05-relations-boundary-drift.md`
   - `docs/cue-authoring-guide.md`
   - `docs/usage.md`
   - `README.md`
2. 说明仿真边界、语料许可证和路径解析规则。
3. 说明 IR `0.1` 和 `0.2` 兼容规则。
4. 说明密钥暴露场景应复用现有攻击标签：
   - `type: "secret_exposure"`
   - `technique: "T1552"`
   - `tactic: "credential_access"`
5. 明确生成器本身不设置攻击标签。

## 8. 实施顺序

```text
生成器合约
  -> machine_secret
  -> corpus_choice
  -> 运行时集成
  -> E2E
  -> 文档
```

`machine_secret` 与 `corpus_choice` 可在合约冻结后并行实现。

## 9. 完成标准

本功能在满足以下条件后完成：

- 两个新生成器通过 CUE 和 Rust 双重校验；
- 内置语料和外部语料覆盖可用且离线；
- 固定 seed 下结果可复现；
- 新生成器不改变 context、template、machine 或输出架构；
- 新生成器不自动设置攻击标签；
- 未使用新功能时现有场景和输出保持兼容；
- IR 版本规则明确并经过测试；
- 语料来源、许可证和处理方式可追溯；
- 单元、集成、端到端和 golden 测试全部通过。
