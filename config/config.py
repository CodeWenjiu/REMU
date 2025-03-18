import toml
from kconfiglib import Kconfig

def main():
    kconf = Kconfig("Kconfig")  # 指定 Kconfig 文件路径
    kconf.load_config(".config")  # 加载 .config 文件
    config_dict = {}

    for sym in kconf.defined_syms:
        if sym.str_value:
            config_dict[sym.name] = sym.str_value

    with open("config.toml", "w") as f:
        toml.dump(config_dict, f)

if __name__ == "__main__":
    main()