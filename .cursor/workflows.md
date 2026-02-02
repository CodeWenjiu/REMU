## 构建

构建工具使用 cargo，入口为 just，规则见根目录 justfile。

可用于检查是否有编译错误：
```nu
just build
```

## 以release模式运行
```nu
just run <主函数参数>
```

## 以debug模式运行
```nu
just dev <主函数参数>
```

## 运行 benchmark
```bash
just bench <CRATE> <BENCH>
```
CRATE 不带 remu_ 前缀（just 会自动加），例如：
```bash
just bench state bus_write
```
