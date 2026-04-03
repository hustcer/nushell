# Bug report: `where $cond` 触发的 parser scope 泄漏导致模块导出丢失

## 现象

在 Nushell `main` 分支上，`ef1ee9393` 构建出的二进制出现回归，而 `67c15500` 仍然正常。
通过 `git bisect` 确认，问题由 `f7c7e63a6d3a37bff474d478351f8439eaeeb585` 引入。

## 最小复现

当前仓库里增加了两个最小文件：

`mod.nu`：

```nu
#!/usr/bin/env nu

export def helper [] {
  let cond = {|x| true }
  [{a: 1}] | where $cond
}

export def main [] {
  'ok'
}
```

`test.nu`：

```nu
#!/usr/bin/env nu

overlay use mod.nu
scope commands
  | where name in [helper mod]
  | select name
  | to nuon --raw
```

复现命令：

```bash
/Users/hustcer/.cargo/bin/nu -c 'source test.nu'
```

坏版本输出：

```text
[]
```

正常输出应该是：

```text
[[name];[helper],[mod]]
```

实际验证结果：

```bash
/Users/hustcer/.cargo/bin/nu -c 'source test.nu'
# []

/Users/hustcer/Applications/nu-nightly/nu -c 'source test.nu'
# [[name];[helper],[mod]]

./target/debug/nu -c 'source test.nu'
# [[name];[helper],[mod]]
```

这个例子说明问题不在 `helper` 自身执行时报错，而是在 `overlay use mod.nu` 之后，模块里本该导出的命令已经“消失”了。

## 前因后果

### 1. 触发条件

触发条件并不复杂：模块中只要有类似下面的代码就可能触发问题：

```nu
let cond = {|x| true }
[{a: 1}] | where $cond
```

关键点不是 `where`，而是 `where` 使用了“提前保存到变量里的闭包/条件”，也就是 `where $cond` 这种形式。

### 2. 出问题的代码路径

问题出在 [`crates/nu-parser/src/parser.rs`](nushell/crates/nu-parser/src/parser.rs) 的 `parse_row_condition()`。

这个函数一开始会执行：

```rust
working_set.enter_scope();
```

目的是为 row condition 临时引入一个新的 `$it` 变量作用域，避免和外层已有的 `$it` 冲突。

随后它会把 `where` 后面的表达式交给 `parse_math_expression()` 解析。

当输入是 `where $cond` 这类表达式时，解析结果不是基于临时 `$it` 构造的新 row condition block，而是一个对现有变量的引用：

- `Expr::Var(arg_var_id) if arg_var_id != var_id`
- 或 `Expr::FullCellPath(...) if ... != var_id`

这两个分支之前都会直接 `return expression;`。

### 3. 真正的根因

原来的实现里，这两个提前返回分支没有执行：

```rust
working_set.exit_scope();
```

此外，`where` 条件类型不匹配时，函数也会在报错后提前返回 `Expression::garbage(...)`，这条错误路径同样没有做 `exit_scope()`。

因此 `parse_row_condition()` 在某些路径上 `enter_scope()` 了，却没有成对 `exit_scope()`。

这会把 parser 的作用域状态泄漏到后续解析流程里，污染 `StateWorkingSet`。

### 4. 为什么最后表现成“模块导出丢失”

这个泄漏不是立刻让 `where $cond` 本身报错，而是让“后续仍在同一个文件里的解析过程”处于错误的 scope 栈状态。

在模块文件解析过程中，后面的 `export def main [] { ... }`、`export use ...`、overlay 导入等声明都依赖 `working_set` 的作用域和可见性信息来建立定义、导出和注册关系。

一旦 scope 栈被破坏，后续模块内容的解析和收集就会偏离预期，表现为：

- 模块命令没有正确进入导出表
- `overlay use mod.nu` 后 `scope commands` 看不到应有的导出
- 真实场景里 `overlay use artifact.nu; artifacts -h` 直接提示 `artifacts` not found

所以表面看是“命令没了”，根因其实是 parser 在解析 `where $cond` 时留下了一个脏作用域。

## 修复方案

修复点很小，位于 [`crates/nu-parser/src/parser.rs`](nushell/crates/nu-parser/src/parser.rs) 的 `parse_row_condition()`。

在这些提前返回分支前补上：

```rust
working_set.exit_scope();
```

也就是保证所有 `enter_scope()` 路径都能严格成对退出，包括错误返回路径。

修复后的逻辑是：

1. 进入临时 `$it` scope
2. 解析 row condition
3. 如果命中 `where $cond` 这类“直接引用已有变量”的提前返回路径，先 `exit_scope()` 再返回
4. 如果类型检查失败，需要返回错误表达式，也先 `exit_scope()`
5. 如果走正常分支构造 `RowCondition` block，最后同样 `exit_scope()`

这次修复不改变语义，只修正 parser 内部状态管理。

## 当前分支上的补充内容

除了 parser 修复，当前分支还增加了两个回归测试：

- [`tests/overlays/mod.rs`](nushell/tests/overlays/mod.rs)
  - `add_overlay_from_file_with_stored_where_condition`
- [`tests/modules/mod.rs`](nushell/tests/modules/mod.rs)
  - `module_public_import_decl_with_stored_where_condition`

这两个测试分别覆盖：

- overlay 从文件导入时，模块里出现 `where $cond` 仍然能正确导出命令
- `export use` 公共导入路径下，模块导出不会因为 `where $cond` 失效

## 结论

这是一个典型的 parser 状态管理回归：

- `f7c7e63a6d3a37bff474d478351f8439eaeeb585` 引入了 `parse_row_condition()` 的提前返回路径
- 这些路径遗漏了 `working_set.exit_scope()`
- 导致 `StateWorkingSet` 作用域泄漏
- 进而污染模块后续解析，最终表现为 overlay / module 导出丢失
- 在当前分支补齐所有提前返回路径的 `exit_scope()` 后，最小复现和真实 `artifact.nu` 场景都已经恢复正常
