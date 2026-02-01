### 注释和文档
由于目前项目正在快速演进，我认为任意形式的注释和文档暂时都是没有意义的，因此请暂时不要添加任何的注释

### 分层处理
很多crate都有option和command这两个文件，分别定义了需要在当前处理的主函数参数和命令，option通过flatten转给上层，而command作为subcommand作为上层的子成员，上层不需要关注底层指令的细节，只需要注意在match到对应项后将subcmd转给下层，每一层只处理与自己相关的工作

### 性能导向
State本来应该作为debugger的子成员而非simulator的自成员，但毕竟性能瓶颈在simulator上，因此最后还是选择将state作为simulator的子成员，state中大量使用了unsafe的uncheck内存访问和inline函数，为了进一步降低访存瓶颈，另外还打算大量使用泛型来做到最高效率的单例化代码

### 前后端解耦
最上层定义main函数的crate,目前来说就是remu-cli，只负责定义用户交互的内容，比如reedline的高亮和补全，同时定义tracer给下层调用，下层只知道有一个动态的实现了tracer trait的对象，在必要的时候调用这个对象，将信息传递，而前端是决定tracer具体行为的crate,将决定tracer会将拿到的信息进行怎样的显示，也就是画画。除此之外，每个抽象层也尽可能的保证上下层之间的解耦

### Policy / Profile 与 Observer

- **Policy**：编译期可定的「配置」抽象为 trait，承载 ISA、位宽、是否 difftest 等；每一层有自己的 Policy（如 `DebuggerPolicy`、`SimulatorPolicy`）。
- **Profile**：承载 Policy 的空 struct，用作泛型参数；例如 `DebuggerProfile<ISA, SimPolicy>`、`SimulatorFastProfile<ISA>`。
- **类型关系**：`DebuggerPolicy::SimPolicy: SimulatorPolicy`；`SimulatorPolicy` 提供 `type ISA: RvIsa` 与 `type Observer: BusObserver`。Debugger 只表达「要哪种 Simulator 行为」，不关心 ISA 具体类型；Simulator 根据 Policy 在类型层面选定 ISA 与 Observer。
- **Observer**：由 `SimulatorPolicy::Observer` 决定。不开启 difftest 时用 `FastObserver`（`ENABLED = false`，DCE 零开销）；开启 difftest 时用 `MmioObserver` 等，用于在 MMIO 时与 ref 同步。Observer 在 remu_state 的 bus 访问中回调（如 `on_mmio_write_*`），不引入运行时分支。

### ISA 与泛型的组装

- **原则**：ISA 与「是否 difftest」等不应由 debugger 关心，应由「谁提供 Simulator 的 Profile 矩阵」来决定。因此将「根据 option.isa（及后续 option.difftest）选择具体 Profile 并调用 `runner.run::<P>(option)`」的 match 放在 **remu_boot**。
- **remu_boot**：仅依赖 remu_debugger、remu_simulator、remu_types；提供 `boot(option, runner)`，内部根据 option 做 match，选出 `DebuggerProfile<ISA, SimulatorXxxProfile<ISA>>` 后调用 `runner.run::<P>(option)`。新增 ISA 或新 Observer 时只改 remu_boot（及 simulator 中新增 Profile 类型），debugger 保持与 ISA 无关。
