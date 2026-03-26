# `input list` / `input` 在半交互终端环境下缺少 TTY 前置校验

## 问题概述

在 Git hook、脚本包装器、CI、`script`/`expect` 等“半交互终端”场景下，Nushell 的交互式输入命令会错误地进入 raw mode 和事件循环：

- `input list`
- `input`
- `input --reedline`

当 `stdout` 或 `stderr` 仍然连接到终端，但 `stdin` 不是 TTY 时，这些命令不会在入口处直接报出清晰错误，而是：

1. 先尝试渲染交互界面；
2. 再在底层事件读取阶段失败；
3. 最终向上抛出较模糊的错误，例如：

```text
Error: nu::shell::io::other

  × I/O error
  ╰─▶   × Interact error, could not process options
```

这会误导使用者，让人误以为是参数错误、列表渲染错误或 hook 配置错误，而不是终端环境不满足要求。

## 典型触发场景

一个实际场景是 Git hook 中调用 Nushell 脚本，并在脚本里使用：

```nu
let iteration = $iters | input list 'Select an iteration'
```

即使 hook 配置了交互模式，实际运行时也可能出现：

- `stdout` 或 `stderr` 是 TTY
- `stdin` 不是 TTY

此时 `input list` 会直接报错。

## 最小复现

### 复现 `input list`

在 macOS / Linux 下可用下面的命令稳定复现：

```sh
script -q /dev/null sh -lc 'nu -c "print (is-terminal --stdin); print (is-terminal --stdout); [2509 2506 0330 0130 1130] | input list Select" </dev/null'
```

预期前两行通常是：

```text
false
true
```

随后会出现一帧列表 UI，再报错：

```text
Error: nu::shell::io::other

  × I/O error
  ╰─▶   × Interact error, could not process options
```

### 复现 `input`

```sh
script -q /dev/null sh -lc 'nu -c "print (is-terminal --stdin); print (is-terminal --stdout); input \"prompt\"" </dev/null'
```

同样会看到：

- `stdin = false`
- `stdout = true`

然后 `input` 报 I/O 相关错误，而不是明确提示 `stdin` 不可交互。

### 复现 `input --reedline`

```sh
script -q /dev/null sh -lc 'nu -c "print (is-terminal --stdin); print (is-terminal --stdout); input --reedline \"prompt\"" </dev/null'
```

该场景下也会因为没有在入口检查 TTY 而进入不可用状态。

## 实际结果

`input list` 当前实现会直接进入：

- `enable_raw_mode()`
- `terminal::size()`
- `event::poll(...)`
- `event::read()`

等交互流程。

在当前源码中，`input list` 的入口没有检查 `stdin` / 输出流是否为 TTY：

- `crates/nu-command/src/platform/input/list.rs`
- `crates/nu-command/src/platform/input/legacy_input.rs`
- `crates/nu-command/src/platform/input/input_.rs`

`nu` 的 REPL 本身已经有类似保护：

- `crates/nu-cli/src/repl.rs`

里面有：

```rust
if !std::io::stdin().is_terminal() {
    return Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "Nushell launched as a REPL, but STDIN is not a TTY; either launch in a valid terminal or provide arguments to invoke a script!",
    ))
    .into_diagnostic();
}
```

但 `input` / `input list` 没有做同类检查。

## 期望结果

对于交互命令，应在进入 raw mode 之前做前置校验，并返回清晰、可诊断的错误，例如：

```text
`input list` requires an interactive terminal on stdin, but stdin is not a TTY
```

或：

```text
`input` requires an interactive terminal on stdout, but stdout is not a TTY
```

这样调用方可以明确知道问题是终端环境，而不是命令参数或渲染逻辑。

## 问题判断

这属于 Nushell 交互输入命令的健壮性问题，具体表现为：

1. 缺少 TTY 前置校验；
2. 错误信息不准确；
3. 在半交互环境里会先进入 UI 渲染，再在更深层失败。

从使用者角度看，这是一个上游 bug。

## 建议修复方案

在以下入口统一增加终端检查：

- `input list`
  - 要求 `stdin` 为 TTY
  - 同时要求实际渲染使用的输出流为 TTY
- `input`
  - legacy 模式要求 `stdin` / `stdout` 为 TTY
  - `--reedline` 模式至少要求 `stdin` / `stdout` 为 TTY

建议行为：

1. 在命令入口直接失败，不进入 raw mode；
2. 使用明确错误文案，而不是最终落成 `Interact error, could not process options`；
3. 最好在不同命令之间统一错误风格。

## 本地修复草案

本地已在源码中加入前置校验，未编译验证，仅作为修复方向记录：

- `crates/nu-command/src/platform/input/mod.rs`
- `crates/nu-command/src/platform/input/list.rs`
- `crates/nu-command/src/platform/input/legacy_input.rs`
- `crates/nu-command/src/platform/input/input_.rs`

修复思路是：

1. 在 `mod.rs` 中集中封装 `require_stdin_terminal` / `require_stdout_terminal` / `require_stderr_terminal`；
2. 在 `input list`、`input` legacy、`input --reedline` 的入口处先校验；
3. 若不满足条件，返回 `ShellError::UnsupportedInput`，给出明确提示。

## 影响范围

可能受影响的场景包括：

- Git hooks
- pre-commit / prepare-commit-msg / commit-msg
- CI 中伪交互终端包装
- `script` / `expect` / PTY 包装器
- IDE / GUI Git 客户端
- 某些 terminal multiplexer / wrapper 场景

## 建议 issue 标题

```text
input list and input should fail fast with a clear TTY error instead of entering raw mode in non-interactive stdin environments
```

## 建议附带信息

提 issue 时建议附上：

- `nu --version`
- 操作系统信息
- 最小复现命令
- `is-terminal --stdin` / `--stdout` / `--stderr` 的输出
- 实际报错完整内容

