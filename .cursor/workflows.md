## 构建

构建工具使用cargo,不过调用者是just,具体规则查看根目录下的justfile

## 构建
可用于检查是否有编译错误
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

## 运行benchmark
```nu
bench <CRATE_NAME> <BENCH_NAME>
```
注意crate_name会自动忽视remu_前缀，比如可以这样
```nu
bench state bus_write
```
