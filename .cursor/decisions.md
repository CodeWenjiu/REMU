### 注释和文档
由于目前项目正在快速演进，我认为任意形式的注释和文档暂时都是没有意义的，因此请暂时不要添加任何的注释

### 分层处理
很多crate都有option和command这两个文件，分别定义了需要在当前处理的主函数参数和命令，option通过flatten转给上层，而command作为subcommand作为上层的子成员，上层不需要关注底层指令的细节，只需要注意在match到对应项后将subcmd转给下层，每一层只处理与自己相关的工作

### 性能导向
State本来应该作为debugger的子成员而非simulator的自成员，但毕竟性能瓶颈在simulator上，因此最后还是选择将state作为simulator的子成员，state中大量使用了unsafe的uncheck内存访问和inline函数，为了进一步降低访存瓶颈，另外还打算大量使用泛型来做到最高效率的单例化代码

### 前后端解耦
最上层定义main函数的crate,目前来说就是remu-cli，只负责定义用户交互的内容，比如reedline的高亮和补全，同时定义tracer给下层调用，下层只知道有一个动态的实现了tracer trait的对象，在必要的时候调用这个对象，将信息传递，而前端是决定tracer具体行为的crate,将决定tracer会将拿到的信息进行怎样的显示，也就是画画。除此之外，每个抽象层也尽可能的保证上下层之间的解耦

### Policy / Profile 与 Observer

- **Policy**：编译期可定的「配置」抽象为 trait，承载 ISA、位宽、是否 difftest 等。层级与模块边界一致：`StatePolicy`（state 层，提供 `type ISA`、`type Observer`）→ `SimulatorPolicy: StatePolicy`（simulator 层）→ `HarnessPolicy: SimulatorPolicy`（harness 层，空子 trait）。Debugger 直辖 Harness，故 Debugger 的泛型约束用 `HarnessPolicy`（由 remu_harness 定义，remu_debugger 再导出）。
- **Profile**：承载 Policy 的空 struct，用作泛型参数；例如 `StateFastProfile<ISA>`、`StateMmioProfile<ISA>`。
- **类型关系**：`SimulatorPolicy` 提供 `type ISA: RvIsa` 与 `type Observer: BusObserver`。Debugger 只表达「要哪种 Harness 行为」，不关心 ISA 具体类型；Harness / Simulator 根据 Policy 在类型层面选定 ISA 与 Observer。
- **Observer**：由 `SimulatorPolicy::Observer` 决定。不开启 difftest 时用 `FastObserver`（`ENABLED = false`，DCE 零开销）；开启 difftest 时用 `MmioObserver` 等，用于在 MMIO 时与 ref 同步。Observer 在 remu_state 的 bus 访问中回调（如 `on_mmio_write_*`），不引入运行时分支。

### ISA 与泛型的组装

- **原则**：ISA 与「是否 difftest」等不应由 debugger 关心，应由「谁提供 Simulator 的 Profile 矩阵」来决定。因此将「根据 option.isa（及后续 option.difftest）选择具体 Profile 并调用 `runner.run::<P>(option)`」的 match 放在 **remu_boot**。
- **remu_boot**：仅依赖 remu_debugger、remu_harness、remu_state、remu_types；提供 `boot(option, runner)`，内部用**两次 match** 避免组合爆炸：第一次 match 按 `option.isa.0` 决定第一个泛型 ISA，调用 `boot_with_isa::<ISA, R>(option, runner)`；第二次 match 在 `boot_with_isa` 内按 `option.difftest` 决定 (P, R)，调用 `runner.run::<P, R>(option)`。新增 ISA 只改第一处 match，新增 difftest 只改第二处 match，debugger 保持与 ISA 无关。

### 泛型与 Policy 约定

- **谁用 Policy 泛型 P**：只在上层「组装层」用。具体：`State<P>`、`SimulatorRemu<P>`、`DecodedInst<P>`、顶层 `decode::<P>`、`Debugger<P, R>`、以及 Harness 的 DUT 类型通过 `D::Policy` 与 P 对应。这些类型或函数统一只带 `P: StatePolicy`、`P: SimulatorPolicy` 或 `P: HarnessPolicy`（按所在层），不在同一处再写 ISA / Observer 泛型。
- **谁用 ISA / Observer 泛型 I、O**：只作为 State 内部实现细节。`Bus<I: RvIsa, O: BusObserver>`、`RiscvReg<I: RvIsa>` 仅由 `State<P>` 在构造时拆开：`Bus<P::ISA, P::Observer>`、`RiscvReg<P::ISA>`。除 state 内部外，不在其他层再写 `I`/`O` 泛型。
- **P 与 I、O 的关系**：唯一拆解点在 `State<P>`：`P::ISA`、`P::Observer` 只在这里用于构造 Bus 和 RiscvReg。其余代码只认 P，不直接依赖 ISA / Observer 类型。
- **Harness 与 Debugger 的 P、D、R**：`Debugger<P, R>` 中 P 为 `HarnessPolicy`、R 为 Ref 模拟器类型；内部为 `Harness<DutSim<P>, R>`，其中 remu_harness 提供 `type DutSim<P> = SimulatorRemu<P, true>`、`type RefSim<P> = SimulatorRemu<P, false>`，使 P 与 DUT/Ref 类型的对应关系显式化。即 Debugger 直辖 Harness，P 为 harness 边界的 Policy（`D::Policy = P`），R 为 Ref 模拟器类型。约定：只在这一层用 P/R（或 D/R），不在此之外再引入一层 Policy 泛型。
- **Simulator 身份（DUT / Ref）**：`SimulatorTrait<P, const IS_DUT: bool>` 通过泛型常量区分身份；D 约束为 `SimulatorTrait<D::Policy, true>`（DUT），R 约束为 `SimulatorTrait<D::Policy, false>`（Ref）。`SimulatorRemu<P, IS_DUT>` 单 impl，可根据 IS_DUT 执行不同行为（如 step_once 中 `if self.func.trace.instruction && IS_DUT` 打 trace，Ref 单例时编译器 DCE 零开销）。该 bool 继续向下传播：`State::new(opt, tracer, is_dut)`、`Bus::new(opt, tracer, is_dut)`，调用处传入常量 `IS_DUT`；Bus 在 new 中根据 `is_dut` 以 `[DUT]` 或 `[REF]` 为前缀打 log；仅 DUT 初始化设备（`is_dut` 为 true 时创建设备列表），Ref 不仿真设备（`is_dut` 为 false 时 device 为空）。
- **ISA 派生类型的可扩展约定**：由 ISA（或 ArchConfig/Extension）推导出的「状态类型」等，一律在 `RvIsa` 上增加关联类型命名（如 `FprState`），调用方使用 `I::FprState`，不在多处写长链。新增类似需求时（如其他扩展的状态类型），在 remu_types 的 `RvIsa` 上增加新关联类型，并在各 ISA 的 impl 中指定具体类型。
