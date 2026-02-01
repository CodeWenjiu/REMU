# Difftest

目前，我们已经实现了一个基本的riscv虚拟机，能够正常运行了，但是我们如何对其进行测试呢？综合成本和准确性来看，最好的测试方法是difftest,具体来说，我们安排一个绝对正确的参考实现ref,将当前待测试的虚拟机视作dut,dut和ref拥有完全相同的初始状态，每运行一条指令，两者就整个寄存器文件进行一轮完整比对，出错则报错，这样就能够精细到具体是哪一条指令出现问题了

## 问题
那么，如何实现这个功能呢？实现难度其实并不高，最大的问题在于性能，difftest会极大的降低模拟器运行的效率，而且哪怕使用状态量在某些情况下关闭difftest,也会带来巨大的开销，比如说
- 在每一条指令运行结束后都检查一次是否需要进行difftest
- 在state模块中需要检查是否存在mmio访问，如果存在，要让ref停止运行一周期并且将dut的整个寄存器文件复制给ref一份，来保证两者同步，因为ref实际上不会仿真设备

## 解决方案
difftest不可能说仿真进行到一般再开启，这一定是通过主函数参数提前决定是否要开启的，换而言之，我们可以专门为是否开启difftest设计两套代码，但是这会带来心智负担，维护负担，而且违反DRY(Don't repeat yourself)原则，所以最好的答案是泛型！

我打算在代码中大量使用泛型，用来处理所有可以提前决定的两并为之在编译期生成性能最好的单例，比如说
- ISA
- 位宽
- 是否开启difftest
等

## 如何实现
既然我们如此大量的使用泛型，那么就不得不考虑不一个好的处理了，我打算沿用decition中对command和option分层处理的哲学，想要让泛型在不同层分层处理，向上层掩盖细节

比如说，我想要的是这样，debugger只需要通过一个类型或者泛型常量向下层simulator表示出“我想要difftest”的意愿，simulator自动将其转化为对应的Obserber,比如说如果不想要difftest,就对应到FastObserver(这个observer实际上不会监视任何事情，这能让代码最快的运行)，如果想要，那可以对应到MMioObserver,监视是否出现了mmio访问

为了实现这个功能，我将单层所有的泛型可能抽象化为一个trait,叫做Policy,为了承载这个trait,需要有一个空的struct,叫做Profile,比如remu_debugger/src/policy.rs中的DebuggerPolicy和DebuggerProfile，类似的，还有SimulatorPolicy,这个具体在Simulator中定义了，这样的话就实现了上层包含下层，因为debugger中的Simulator就可以定义为Simulator<P::SimPolicy>

但是组装仍然有问题，譬如说，ISA这个参数实际上不应该是debugger应该关心的，我的期望是，simulator层根据isa参数自己决定应该匹配什么样的泛型，但是我不知道怎么做，所以目前的做法仍然是在debugger中match isa,然后将不同的类型转给simulator,就像是
```rust
match option.isa.0 {
    Architecture::Riscv32(arch) => match arch {
        Riscv32Architecture::Riscv32i => {
            runner.run::<DebuggerProfile<RV32I, SimulatorFastProfile<RV32I>>>(option)
        }

        Riscv32Architecture::Riscv32im => {
            runner.run::<DebuggerProfile<RV32I, SimulatorFastProfile<RV32IM>>>(option)
        }
        _ => unreachable!(),
    },
    _ => unreachable!(),
}
```
