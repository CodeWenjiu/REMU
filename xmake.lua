-- xmake.lua
set_version("0.0.1")

-- 支持的平台
local platforms = {
    rv32im_emu_dm      = "rv32im-emu-dm",
    rv32im_emu_dm_alias= "riscv32-emu-dm",

    rv32im_emu_sc      = "rv32im-emu-sc",
    rv32im_emu_sc_alias= "riscv32-emu-sc",

    rv32im_emu_pl      = "rv32im-emu-pl",
    rv32im_emu_pl_alias= "riscv32-emu-pl",
    
    rv32e_emu          = "rv32e-emu-dm",

    Nzea_npc           = "rv32e-nzea-npc",
    Nzea_npc_alias     = "riscv32e-nzea-npc",
    
    Nzea_ysyxsoc       = "rv32e-nzea-ysyxsoc",

    Nzea_jyd_remote    = "rv32i-nzea-jyd_remote"
}

-- 默认平台
option("platform")
    set_default(platforms.rv32im_emu_dm)
    set_showmenu(true)
    set_description("Select simulation platform")
    set_values(table.unpack(table.values(platforms)))
option_end()

-- 二进制文件配置
option("binfile")
    set_default("")
    set_showmenu(true)
    set_description("Primary binary file")
option_end()

option("alternate")
    set_default("")
    set_showmenu(true)
    set_description("Alternate binary file")
option_end()

-- 动态 config 文件目标
target("dynamic_config")
    set_kind("phony")
    on_build(function ()
        local configfile = "config/dynamic/.config"
        if not os.isfile(configfile) then
            cprint("${yellow}" .. configfile .. " not found, generating...${clear}")
            os.exec("xmake menuconfig-dynamic-conditional")
        end
    end)

-- 静态 config 文件目标
target("static_config")
    set_kind("phony")
    on_build(function ()
        local configfile = "config/static/.config"
        if not os.isfile(configfile) then
            cprint("${yellow}" .. configfile .. " not found, generating...${clear}")
            os.exec("xmake menuconfig-static-conditional")
        end
    end)

-- 目标定义
target("core")
    set_kind("binary")
    add_files("core/src/main.rs")
    set_rundir("$(projectdir)")
    add_deps("dynamic_config", "static_config")

    -- 传递参数给 cargo
    before_run(function (target)
        import("core.base.option")

        local configfile_dynamic = "config/dynamic/.config"
        local configfile_static = "config/static/.config"
        local args = {}

        local binfile = option.get("binfile")
        if binfile and binfile ~= "" and binfile ~= "nil" then
            table.insert(args, "--primary-bin")
            table.insert(args, path.absolute(binfile))
        end

        local alternate = option.get("alternate")
        if alternate and alternate ~= "" and alternate ~= "nil" then
            table.insert(args, "--additional-bin")
            table.insert(args, path.absolute(alternate))
        end

        local platform = option.get("platform")
        if platform and platform ~= "" and platform ~= "nil" then
            table.insert(args, "-p")
            table.insert(args, platform)
        end

        table.insert(args, "-c")
        table.insert(args, path.absolute(configfile_dynamic))

        -- 支持额外参数
        if option.get("extraargs") then
            table.join2(args, option.get("extraargs"))
        end
        -- 保存到 target:data 供 on_run/on_debug 使用
        target:data_set("runargs", args)

    end)

    on_run(function (target)
        os.execv("cargo", table.join({"run", "--release", "--bin", "core", "--"}, target:data("runargs")))
    end)

    after_run(function (target)
        if is_mode("debug") then
            os.execv("cargo", table.join({"run", "--bin", "core", "--"}, target:data("runargs")))
        end
    end)

    on_clean(function (target)
        os.exec("cargo clean")
    end)

-- 格式化任务
task("fmt")
    set_menu {usage = "xmake fmt", description = "Format all Rust code"}
    on_run(function ()
        os.exec("cargo fmt --all --manifest-path ./Cargo.toml")
    end)

-- 配置相关任务
task("menuconfig-buildin")
    set_menu {usage = "xmake menuconfig-buildin", description = "Run menuconfig"}
    on_run(function ()
        os.exec("make -C ./config menuconfig")
    end)

task("menuconfig-static")
    set_menu {usage = "xmake menuconfig-static", description = "Run menuconfig-static"}
    on_run(function ()
        os.exec("make -C ./config menuconfig-static")
    end)

task("menuconfig-dynamic")
    set_menu {usage = "xmake menuconfig-dynamic", description = "Run menuconfig-dynamic"}
    on_run(function ()
        os.exec("make -C ./config menuconfig-dynamic")
    end)

task("menuconfig-static-conditional")
    set_menu {usage = "xmake menuconfig-static-conditional", description = "Run menuconfig-static-conditional"}
    on_run(function ()
        os.exec("make -C ./config menuconfig-static-conditional")
    end)

task("menuconfig-dynamic-conditional")
    set_menu {usage = "xmake menuconfig-dynamic-conditional", description = "Run menuconfig-dynamic-conditional"}
    on_run(function ()
        os.exec("make -C ./config menuconfig-dynamic-conditional")
    end)

task("config_dependencies")
    set_menu {usage = "xmake config_dependencies", description = "Run all config dependencies"}
    on_run(function ()
        os.exec("xmake menuconfig-static-conditional")
        os.exec("xmake menuconfig-dynamic-conditional")
    end)

-- 其他杂项任务
task("fetch")
    set_menu {usage = "xmake fetch", description = "Show project info with onefetch"}
    on_run(function ()
        os.exec("onefetch")
    end)

task("gource")
    set_menu {usage = "xmake gource", description = "Show git history with gource"}
    on_run(function ()
        os.exec("sh -c \"git log --pretty=format:'%at|%s' --reverse --no-merges > commitmsg.txt\"")
        os.exec("gource --load-config ./gource.ini")
    end)
